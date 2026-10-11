"""Run an installed package from a different cwd with a fresh temporary database."""

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen


def main():
    with tempfile.TemporaryDirectory(prefix="agent-control-smoke-") as directory:
        with socket.socket() as socket_:
            socket_.bind(("127.0.0.1", 0))
            port = socket_.getsockname()[1]
        env = os.environ.copy()
        env.pop("PYTHONPATH", None)
        env.update(
            AGENT_CONTROL_DATA_DIR=str(Path(directory) / "data"),
            AGENT_CONTROL_ORIGIN=f"http://127.0.0.1:{port}",
            AGENT_CONTROL_EXECUTION_ENABLED="false",
        )
        process = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "uvicorn",
                "backend.app.main:app",
                "--host",
                "127.0.0.1",
                "--port",
                str(port),
                "--no-proxy-headers",
            ],
            cwd=directory,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            for _ in range(100):
                if process.poll() is not None:
                    raise RuntimeError("Backend exited before becoming healthy")
                try:
                    with urlopen(f"http://127.0.0.1:{port}/health", timeout=1) as response:
                        result = json.load(response)
                    break
                except URLError:
                    time.sleep(0.1)
            else:
                raise RuntimeError("Backend did not become healthy")
            assert result["service"] == "agent-control" and result["status"] == "ok"
            for path in ["/", "/ui/src/app.js", "/ui/src/styles/app.css", "/ui/favicon.svg"]:
                with urlopen(f"http://127.0.0.1:{port}{path}", timeout=2) as response:
                    assert response.status == 200
            assert (Path(directory) / "data" / "agent-control.sqlite3").exists()
            print(json.dumps({"standalone_start": "passed", "health": result, "frontend_assets": "passed"}))
        finally:
            process.terminate()
            process.wait(timeout=10)


if __name__ == "__main__":
    main()
