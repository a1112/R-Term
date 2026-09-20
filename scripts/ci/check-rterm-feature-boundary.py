"""Verify the real production Cargo graph with SSH disabled and enabled."""
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
ssh_packages = {"russh", "rssh-ssh", "rssh-types", "rterm-ssh-adapter", "rpassword"}
results = []
for name, flags, enabled in (
    ("default-local", [], False),
    ("no-default-features", ["--no-default-features"], False),
    ("ssh-extension", ["--features", "ssh"], True),
):
    command = ["cargo", "tree", "--locked", "-p", "rssh-app", "--edges", "normal,build",
               "--prefix", "none", "--format", "{p}", *flags]
    output = subprocess.check_output(command, cwd=root, text=True)
    packages = {line.split()[0] for line in output.splitlines() if line.strip()}
    present = packages & ssh_packages
    passed = present == ssh_packages if enabled else not present
    results.append({"profile": name, "passed": passed, "ssh_packages": sorted(present),
                    "package_count": len(packages), "command": command})
print(json.dumps({"passed": all(item["passed"] for item in results), "profiles": results}, indent=2))
raise SystemExit(0 if all(item["passed"] for item in results) else 1)
