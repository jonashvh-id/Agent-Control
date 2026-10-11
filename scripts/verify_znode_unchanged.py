"""Read-only comparison of recorded ZNode Git/source state. Never writes in ZNode."""

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def verify():
    baseline = json.loads((ROOT / "docs/migration/znode-before.json").read_text())
    failures = []
    for entry in baseline:
        path = Path(entry["path"])

        def git(*args):
            return subprocess.check_output(["git", "--no-optional-locks", "-C", str(path), *args], text=True)

        current = {
            "head": git("rev-parse", "HEAD").strip(),
            "branch": git("branch", "--show-current").strip(),
            "status": git("status", "--porcelain=v1"),
            "refs": git("show-ref"),
        }
        for field, value in current.items():
            if value != entry[field]:
                failures.append({"checkout": str(path), "field": field})
        files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z").split("\0")
        hashes = {}
        for rel in files:
            if not rel:
                continue
            file = path / rel
            hashes[rel] = hashlib.sha256(file.read_bytes()).hexdigest() if file.is_file() else None
        if hashes != entry["files"]:
            failures.append({"checkout": str(path), "field": "source_file_hashes"})
    result = {
        "unchanged": not failures,
        "checkouts": len(baseline),
        "source_files": sum(len(entry["files"]) for entry in baseline),
        "differences": failures,
    }
    print(json.dumps(result, indent=2))
    return result


if __name__ == "__main__":
    raise SystemExit(0 if verify()["unchanged"] else 1)
