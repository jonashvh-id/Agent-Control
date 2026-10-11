"""Read-only migration verification; private Dovehouse baseline remains outside source control."""
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    snapshot = Path("var/rust-migration/dovehouse-before.json")
    data = json.loads(snapshot.read_text())
    root = Path(data["root"])
    files = subprocess.check_output(["rg", "--files", "--hidden", "-g", "!**/.git/**", "-g", "!**/node_modules/**", "-g", "!**/.venv/**", str(root)]).decode().splitlines()
    current = {str(Path(p).relative_to(root)): hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in files if Path(p).is_file() and not Path(p).is_symlink()}
    differences = ["file_manifest"] if current != data["files"] else []
    for repository in data["git"]:
        for name, args in {"head": ["rev-parse", "HEAD"], "status": ["status", "--porcelain=v1", "--untracked-files=all"], "refs": ["show-ref"]}.items():
            output = subprocess.run(["git", "--no-optional-locks", "-C", repository["path"], *args], capture_output=True, text=True)
            if output.stdout != repository["state"][name]:
                differences.append("git_" + name)
    print(json.dumps({"project": "Dovehouse", "unchanged": not differences, "files": len(current), "repositories": len(data["git"]), "differences": differences}, indent=2))
    raise SystemExit(bool(differences))


if __name__ == "__main__":
    main()
