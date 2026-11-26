# Copyright 2026 Captain James
# SPDX-License-Identifier: Apache-2.0
"""Parley's message types in Python, rendered from the wesley-parley model."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from wesley_python.syntax import Module


class ModelError(ValueError):
    """The model is not the shape ``wesley-parley model`` prints."""


def module_from_model(model: object) -> Module:
    """Build the module of message types for ``model``."""
    raise NotImplementedError(model)


def render_model(model: object) -> str:
    """Render the module of message types for ``model`` as Python source."""
    raise NotImplementedError(model)
