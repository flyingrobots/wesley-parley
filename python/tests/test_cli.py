# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""The command line: ``python -m wesley_parley python [--model FILE] [--out FILE]``."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tests.support import GOLDEN, MODEL, SRC


def _cli(*args: str, stdin: str = "", hash_seed: str = "0") -> subprocess.CompletedProcess[str]:
    env = dict(os.environ, PYTHONPATH=str(SRC), PYTHONHASHSEED=hash_seed)
    return subprocess.run(  # noqa: S603 - runs this interpreter on this package
        [sys.executable, "-m", "wesley_parley", *args],
        input=stdin,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )


def _golden() -> str:
    return (GOLDEN / "messages.py").read_bytes().decode("utf-8")


class CommandLineTest(unittest.TestCase):
    def test_model_from_standard_input(self) -> None:
        result = _cli("python", stdin=MODEL.read_text(encoding="utf-8"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, _golden())
        self.assertEqual(result.stderr, "")

    def test_model_from_a_file(self) -> None:
        result = _cli("python", "--model", str(MODEL))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, _golden())

    def test_output_to_a_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "messages.py"
            result = _cli("python", "--model", str(MODEL), "--out", str(out))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertEqual(out.read_bytes().decode("utf-8"), _golden())

    def test_same_bytes_under_any_hash_seed(self) -> None:
        outputs = {_cli("python", "--model", str(MODEL), hash_seed=seed).stdout for seed in ("0", "1", "4242")}
        self.assertEqual(len(outputs), 1)

    def test_a_missing_model_file_is_an_error(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = _cli("python", "--model", str(Path(tmp) / "absent.json"))
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("wesley-parley-python: error:", result.stderr)

    def test_invalid_json_is_an_error(self) -> None:
        result = _cli("python", stdin="{not json")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("not valid JSON", result.stderr)

    def test_a_malformed_model_is_an_error(self) -> None:
        result = _cli("python", stdin='{"messages": 1, "objects": {}}')
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("messages", result.stderr)

    def test_a_command_is_required(self) -> None:
        result = _cli()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("usage:", result.stderr)


if __name__ == "__main__":
    unittest.main()
