"""Bundle the single frontend source into wheels; never duplicate editable UI sources."""

import shutil
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py


class BuildWithFrontend(build_py):
    def run(self):
        super().run()
        frontend = Path(__file__).parent / "frontend"
        target = Path(self.build_lib) / "backend" / "app" / "web"
        target.mkdir(parents=True, exist_ok=True)
        shutil.copy2(frontend / "index.html", target / "index.html")
        shutil.copy2(frontend / "favicon.svg", target / "favicon.svg")
        shutil.copytree(frontend / "src", target / "src", dirs_exist_ok=True)


setup(cmdclass={"build_py": BuildWithFrontend})
