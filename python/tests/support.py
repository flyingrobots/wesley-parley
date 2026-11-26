# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""Shared helpers for the test suite: paths, the model and the golden file."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import unittest

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
TESTS = ROOT / "tests"
GOLDEN = TESTS / "golden"
TYPECHECK = TESTS / "typecheck"

# The Rust crate's golden model is the single source: its own test keeps it fresh against Parley's SDL.
MODEL = ROOT.parent / "crates" / "wesley-parley" / "tests" / "golden" / "model.json"

UPDATE_ENV = "WESLEY_UPDATE_GOLDEN"


def load_model() -> object:
    """Read the golden model."""
    return json.loads(MODEL.read_text(encoding="utf-8"))


def assert_golden(case: unittest.TestCase, name: str, actual: str) -> None:
    """Compare ``actual`` to ``tests/golden/<name>`` byte for byte; rewrite it when asked to."""
    path = GOLDEN / name
    if os.environ.get(UPDATE_ENV) == "1":
        path.write_bytes(actual.encode("utf-8"))
    case.assertTrue(path.is_file(), f"missing golden {path}; run with {UPDATE_ENV}=1")
    expected = path.read_bytes().decode("utf-8")
    case.assertEqual(expected, actual, f"output differs from golden {path}")
