# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""wesley-parley's Python bindings: Parley's message types as standard-library ``TypedDict`` classes.

The Rust crate interprets Parley's vocabulary once into a model and prints it with
``wesley-parley model``. :mod:`wesley_parley.render` reads that model and builds a module through
wesley-python's syntax API; :mod:`wesley_parley.cli` is its command line.
"""

from __future__ import annotations

__version__ = "0.1.0"
