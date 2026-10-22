"""Check the trial's Cargo boundary, including optional SSH dependencies."""
import argparse
import json
from pathlib import Path
import subprocess


def metadata(root, host):
    result = subprocess.run(["cargo", "metadata", "--locked", "--all-features",
                             "--format-version", "1", "--filter-platform", host],
                            cwd=root, capture_output=True, text=True, check=True)
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ssh", type=Path, required=True)
    parser.add_argument("--gui", type=Path, required=True)
    args = parser.parse_args()
    host = next(line[6:] for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
    ssh, gui = metadata(args.ssh, host), metadata(args.gui, host)
    forbidden = ("rterm-", "wgpu", "winit", "tauri", "cosmic-text", "fontdb", "glyphon", "softbuffer", "arboard")
    violations = []
    expected = {"rssh-cli", "rssh-ssh", "rssh-types", "rssh-test-support"}
    members = {p["name"] for p in ssh["packages"] if p["id"] in ssh["workspace_members"]}
    if members != expected:
        violations.append(f"SSH workspace differs: {sorted(members)}")
    for package in ssh["packages"]:
        if package["name"].startswith(forbidden):
            violations.append(f"SSH depends on GUI package {package['name']}")
    for label, root, graph in [("SSH", args.ssh, ssh), ("GUI", args.gui, gui)]:
        for package in graph["packages"]:
            if package["source"] is None and not Path(package["manifest_path"]).resolve().is_relative_to(root.resolve()):
                violations.append(f"{label} has external path dependency {package['name']}")
    source = json.loads((args.gui / "TRIAL-SOURCE.json").read_text())
    for name in ("rssh-ssh", "rssh-types"):
        packages = [p for p in gui["packages"] if p["name"] == name]
        expected_source = f"git+{source['ssh_git']}?rev={source['ssh_commit']}#{source['ssh_commit']}"
        if len(packages) != 1 or packages[0]["source"] != expected_source:
            violations.append(f"GUI {name} is not from the recorded immutable Git source")
    for path in ("web", "tauri", "crates/rssh-app", "crates/rssh-native", "crates/rssh-runtime",
                 "crates/rssh-terminal", "crates/rterm-fonts", "crates/rterm-render-wgpu"):
        if (args.ssh / path).exists() or not (args.gui / path).exists():
            violations.append(f"wrong ownership for {path}")
    print(json.dumps({"passed": not violations, "ssh_members": sorted(members),
                      "ssh_resolved_packages": len(ssh["packages"]), "gui_resolved_packages": len(gui["packages"]),
                      "ssh_commit": source["ssh_commit"], "violations": violations}, indent=2))
    raise SystemExit(bool(violations))


if __name__ == "__main__":
    main()
