# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""The golden module is checked by the real toolchain: compiled, imported and type-checked."""

from __future__ import annotations

import py_compile
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tests.support import GOLDEN, TYPECHECK

GOLDEN_MODULE = GOLDEN / "messages.py"

# Each fixture that must fail, with the mypy error code and a fragment of the message it must fail with.
EXPECTED_FAILURES = {
    "bad_missing_required.py": ("typeddict-item", 'Missing key "status"'),
    "bad_event.py": ("typeddict-item", "Literal['end']"),
    "bad_null_optional.py": ("typeddict-item", '"dry_run"'),
}


def _run_mypy(workdir: Path) -> list[str]:
    """Run mypy --strict for Python 3.9 over every file in ``workdir``; return its error lines."""
    config = workdir / "mypy.ini"
    config.write_text("[mypy]\n", encoding="utf-8")
    files = sorted(path.name for path in workdir.glob("*.py"))
    result = subprocess.run(  # noqa: S603 - runs mypy from this interpreter's environment
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--python-version",
            "3.9",
            "--config-file",
            str(config),
            "--cache-dir",
            str(workdir / ".mypy_cache"),
            "--no-error-summary",
            "--show-error-codes",
            *files,
        ],
        cwd=workdir,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode not in {0, 1}:
        msg = f"mypy did not run: {result.stdout}{result.stderr}"
        raise AssertionError(msg)
    return [line for line in result.stdout.splitlines() if ": error: " in line]


class GoldenToolchainTest(unittest.TestCase):
    def test_golden_compiles(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            py_compile.compile(str(GOLDEN_MODULE), cfile=str(Path(tmp) / "out.pyc"), doraise=True)

    def test_golden_imports_the_standard_library_only(self) -> None:
        namespace = runpy.run_path(str(GOLDEN_MODULE), run_name="messages")
        modules = {getattr(value, "__module__", None) for value in namespace.values()}
        self.assertLessEqual(modules - {None}, {"messages", "typing", "builtins"})

    def test_required_and_optional_keys_at_runtime(self) -> None:
        namespace = runpy.run_path(str(GOLDEN_MODULE), run_name="messages")
        self.assertEqual(namespace["QueryWindow"].__required_keys__, frozenset({"from", "to"}))
        self.assertIn("in", namespace["Redaction"].__required_keys__)
        self.assertIn("dry_run", namespace["RunBootstrap"].__optional_keys__)
        self.assertEqual(namespace["EndSchemaRef"].__required_keys__, frozenset({"id", "version"}))


class MypyTest(unittest.TestCase):
    """mypy --strict, for Python 3.9, over the golden and the fixtures that use it."""

    errors: list[str]

    @classmethod
    def setUpClass(cls) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            workdir = Path(tmp)
            for path in [GOLDEN_MODULE, *sorted(TYPECHECK.glob("*.py"))]:
                shutil.copy(path, workdir / path.name)
            cls.errors = _run_mypy(workdir)

    def _errors_in(self, name: str) -> list[str]:
        return [line for line in self.errors if line.startswith(f"{name}:")]

    def test_golden_and_good_uses_are_clean(self) -> None:
        for name in ("messages.py", "uses_messages.py"):
            with self.subTest(file=name):
                self.assertEqual(self._errors_in(name), [])

    def test_every_bad_fixture_fails_with_its_expected_error(self) -> None:
        on_disk = {path.name for path in TYPECHECK.glob("bad_*.py")}
        self.assertEqual(on_disk, set(EXPECTED_FAILURES))
        for name, (code, fragment) in EXPECTED_FAILURES.items():
            with self.subTest(file=name):
                errors = self._errors_in(name)
                self.assertEqual(len(errors), 1, f"mypy reported {errors} for {name}")
                self.assertIn(f"[{code}]", errors[0])
                self.assertIn(fragment, errors[0])


if __name__ == "__main__":
    unittest.main()
