# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""The command line: ``python -m wesley_parley python [--model FILE] [--out FILE]``."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def main(argv: Sequence[str] | None = None) -> int:
    """Run the command line and return its exit status: 0 on success, 1 on any error."""
    raise NotImplementedError(argv)
