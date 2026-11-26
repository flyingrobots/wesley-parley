# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""The command line: ``python -m wesley_parley python [--model FILE] [--out FILE]``."""

from __future__ import annotations

import argparse
import json
import sys
from typing import TYPE_CHECKING

from wesley_parley.render import render_model

if TYPE_CHECKING:
    from collections.abc import Sequence

PROG = "wesley-parley-python"


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=PROG, description="Render Parley's message types from the wesley-parley model."
    )
    commands = parser.add_subparsers(dest="command", required=True)
    python = commands.add_parser(
        "python",
        help="render the message types as standard-library Python TypedDicts",
        description="Read the model `wesley-parley model` prints and write a Python module of TypedDicts.",
    )
    python.add_argument("--model", metavar="FILE", help="the model JSON to read (default: standard input)")
    python.add_argument("--out", metavar="FILE", help="the module to write (default: standard output)")
    return parser


def _python(model_path: str | None, out_path: str | None) -> None:
    if model_path is None:
        text = sys.stdin.read()
    else:
        with open(model_path, encoding="utf-8") as handle:  # noqa: PTH123 - pathlib is not needed
            text = handle.read()
    output = render_model(json.loads(text))
    if out_path is None:
        sys.stdout.write(output)
    else:
        with open(out_path, "w", encoding="utf-8", newline="\n") as handle:  # noqa: PTH123
            handle.write(output)


def main(argv: Sequence[str] | None = None) -> int:
    """Run the command line and return its exit status: 0 on success, 1 on any error."""
    args = _parser().parse_args(argv)
    try:
        _python(args.model, args.out)
    except json.JSONDecodeError as error:
        sys.stderr.write(f"{PROG}: error: the model is not valid JSON: {error}\n")
        return 1
    except (OSError, ValueError) as error:
        sys.stderr.write(f"{PROG}: error: {error}\n")
        return 1
    return 0
