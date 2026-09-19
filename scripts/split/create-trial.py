"""Create local trial repositories from one committed source; never publish them.

Both outputs retain the original Git ancestry. This is not a history-filtered
release extraction and does not advance the Stage 7 certification gate.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess


def run(*args, cwd):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def write(root, path, text):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    if output.exists() or output == source or source in output.parents:
        parser.error("output must be a new directory outside the source checkout")
    revision = run("git", "rev-parse", "HEAD", cwd=source)
    output.mkdir(parents=True)
    ssh, gui = output / "R-SSH", output / "R-Term"
    for repo in (ssh, gui):
        run("git", "clone", "--no-hardlinks", "--no-checkout", str(source), str(repo), cwd=source)
        run("git", "remote", "remove", "origin", cwd=repo)
        run("git", "switch", "--orphan", "codex/trial-output", cwd=repo)
        # Seed the index with the exact source commit; checkout only owned paths.
        run("git", "read-tree", revision, cwd=repo)
        run("git", "symbolic-ref", "HEAD", "refs/heads/codex/trial-split", cwd=repo)
        run("git", "update-ref", "HEAD", revision, cwd=repo)
    run("git", "checkout-index", "--all", cwd=gui)
    owned = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "LICENSE", "NOTICE",
             ".gitignore", ".gitattributes", ".editorconfig", "crates/rssh-ssh",
             "crates/rssh-test-support"]
    run("git", "restore", "--source", revision, "--worktree", "--", *owned, cwd=ssh)
    root_manifest = (ssh / "Cargo.toml").read_text(encoding="utf-8")
    root_manifest = re.sub(r"members = \[.*?\]", 'members = ["crates/rssh-types", "crates/rssh-ssh", "crates/rssh-test-support", "crates/rssh-cli"]', root_manifest, count=1, flags=re.S)
    root_manifest = re.sub(r"exclude = .*\n", "", root_manifest)
    root_manifest = root_manifest.split("[patch.crates-io]")[0]
    write(ssh, "Cargo.toml", root_manifest)
    types = (source / "crates/rterm-types/src/lib.rs").read_text(encoding="utf-8")
    size_start = types.index("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct TerminalSize")
    size_end = types.index("#[derive", size_start + 10)
    size = types[size_start:size_end]
    write(ssh, "crates/rssh-types/src/lib.rs", "//! Dependency-free SSH PTY dimensions.\n\n" + size)
    write(ssh, "crates/rssh-types/Cargo.toml", (source / "crates/rterm-types/Cargo.toml").read_text().replace('name = "rterm-types"', 'name = "rssh-types"'))
    manifest = (ssh / "crates/rssh-ssh/Cargo.toml").read_text()
    manifest = manifest.replace('[features]\nruntime-adapter = ["dep:rterm-runtime"]\n\n', '')
    manifest = re.sub(r"^rterm-runtime = .*\n|^rssh-domain = .*\n", "", manifest, flags=re.M)
    manifest = manifest.replace("rterm-types", "rssh-types")
    manifest = re.sub(r'\[\[test\]\]\nname = "runtime_adapter".*?(?=\[lints\])', "", manifest, flags=re.S)
    write(ssh, "crates/rssh-ssh/Cargo.toml", manifest)
    for relative in ["src/lib.rs", "src/russh_client.rs", "tests/loopback_native.rs"]:
        path = "crates/rssh-ssh/" + relative
        text = (ssh / path).read_text().replace("rterm_types", "rssh_types")
        text = text.replace('#[cfg(feature = "runtime-adapter")]\nmod runtime_adapter;\n', '')
        text = text.replace('#[cfg(feature = "runtime-adapter")]\npub use runtime_adapter::{SshControl, SshInterrupt, SshReader, SshTransport, SshWriter};\n', '')
        write(ssh, path, text)
    # These files belong to the GUI-owned transport adapter, not the SSH backend.
    for path in ["crates/rssh-ssh/src/runtime_adapter.rs", "crates/rssh-ssh/tests/runtime_adapter.rs"]:
        (ssh / path).unlink()
    for path in (source / "scripts/split/ssh-cli").rglob("*"):
        if path.is_file():
            write(ssh, "crates/rssh-cli/" + path.relative_to(source / "scripts/split/ssh-cli").as_posix(), path.read_text())
    write(ssh, "README.md", (source / "scripts/split/SSH-README.md").read_text())
    write(ssh, "TRIAL-SOURCE.json", json.dumps({"source_commit": revision, "role": "ssh-only", "certified": False}, indent=2) + "\n")
    run("cargo", "metadata", "--offline", "--format-version", "1", cwd=ssh)
    run("cargo", "fmt", "--all", cwd=ssh)
    run("git", "add", "-A", cwd=ssh)
    run("git", "commit", "-m", "split(trial): isolate SSH backend and command-line client", cwd=ssh)
    ssh_revision = run("git", "rev-parse", "HEAD", cwd=ssh)
    git_source = ssh.as_uri()
    dependency = f'git = "{git_source}", rev = "{ssh_revision}"'
    # Preserve existing GUI call sites behind a GUI-owned adapter facade.
    adapter_manifest = '''[package]
name = "rterm-ssh-adapter"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
version.workspace = true

[lib]
name = "rssh_ssh"

[features]
runtime-adapter = []

[dependencies]
ssh-core = { package = "rssh-ssh", DEPENDENCY }
rterm-runtime = { path = "../rssh-runtime", version = "0.1.0" }
rterm-types = { path = "../rterm-types", version = "0.1.0" }

[dev-dependencies]
rssh-domain = { path = "../rssh-domain", version = "0.1.0" }

[lints]
workspace = true
'''.replace("DEPENDENCY", dependency)
    write(gui, "crates/rssh-ssh/Cargo.toml", adapter_manifest)
    write(gui, "crates/rssh-ssh/src/lib.rs", "//! GUI-owned bridge from the standalone SSH backend to the terminal runtime.\npub use ssh_core::*;\nmod runtime_adapter;\npub use runtime_adapter::{SshControl, SshInterrupt, SshReader, SshTransport, SshWriter};\n")
    for path in ["crates/rssh-ssh/src/russh_client.rs", "crates/rssh-ssh/tests/loopback_native.rs"]:
        (gui / path).unlink()
    types = types[:size_start] + "pub use rssh_types::TerminalSize;\n\n" + types[size_end:]
    write(gui, "crates/rterm-types/src/lib.rs", types)
    path = "crates/rterm-types/Cargo.toml"
    write(gui, path, (gui / path).read_text().replace("[dependencies]", '[dependencies]\nrssh-types = { ' + dependency + ' }'))
    path = "crates/rssh-app/Cargo.toml"
    write(gui, path, (gui / path).read_text().replace('rssh-ssh = { path', 'rssh-ssh = { package = "rterm-ssh-adapter", path'))
    write(gui, "README.md", (source / "scripts/split/GUI-README.md").read_text())
    write(gui, "TRIAL-SOURCE.json", json.dumps({"source_commit": revision, "role": "graphical-terminal", "ssh_commit": ssh_revision, "ssh_git": git_source, "certified": False}, indent=2) + "\n")
    # Legacy workflows describe the monolith; retain them as documentation only.
    for workflow in (gui / ".github/workflows").glob("*.yml"):
        write(gui, "docs/trial/legacy-workflows/" + workflow.name, workflow.read_text())
        workflow.unlink()
    run("cargo", "metadata", "--offline", "--format-version", "1", cwd=gui)
    run("cargo", "fmt", "--all", cwd=gui)
    run("git", "add", "-A", cwd=gui)
    run("git", "commit", "-m", "split(trial): move complete graphical product to R-Term", cwd=gui)
    manifest = {"source_commit": revision, "ssh": {"path": str(ssh), "commit": ssh_revision}, "gui": {"path": str(gui), "commit": run("git", "rev-parse", "HEAD", cwd=gui)}, "certified": False, "history": "original ancestry retained; not filtered or publication-audited"}
    write(output, "trial.json", json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
