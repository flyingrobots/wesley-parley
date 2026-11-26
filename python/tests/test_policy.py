# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""Repository rules checked over the package source: no code built from strings."""

from __future__ import annotations

import ast
import re
import unittest

from tests.support import SRC

# Crude, on purpose: a string that looks like generated Python means someone is templating.
TEMPLATE_PATTERNS = (re.compile(r"TypedDict\("), re.compile(r"class\s+[A-Za-z_]\w*\s*\("))


def _sources() -> list[tuple[str, ast.Module]]:
    return [
        (str(path.relative_to(SRC)), ast.parse(path.read_text(encoding="utf-8"), filename=str(path)))
        for path in sorted(SRC.rglob("*.py"))
    ]


def _template_hits(tree: ast.Module) -> list[str]:
    return [
        node.value
        for node in ast.walk(tree)
        if isinstance(node, ast.Constant)
        and isinstance(node.value, str)
        and any(pattern.search(node.value) for pattern in TEMPLATE_PATTERNS)
    ]


class NoTemplatingTest(unittest.TestCase):
    def test_there_are_sources(self) -> None:
        self.assertGreaterEqual(len(_sources()), 3)

    def test_no_string_constant_looks_like_generated_code(self) -> None:
        for name, tree in _sources():
            with self.subTest(file=name):
                self.assertEqual(_template_hits(tree), [])

    def test_the_guard_catches_a_template(self) -> None:
        tree = ast.parse("CODE = 'class End(_EndRequired):'\nOTHER = \"X = TypedDict('X', {})\"")
        self.assertEqual(len(_template_hits(tree)), 2)


if __name__ == "__main__":
    unittest.main()
