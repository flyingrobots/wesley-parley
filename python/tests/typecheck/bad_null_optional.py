"""An optional key is absent when unset, never null: ``None`` in ``dry_run`` must fail mypy."""

from __future__ import annotations

from messages import RunBootstrap

boot: RunBootstrap = {
    "event": "run",
    "t": 1,
    "seq": 0,
    "tool": "count",
    "v": "0.1.0",
    "run": "r",
    "paths": {"receipt": "/r.jsonl"},
    "_schema": {"id": "parley.run.bootstrap", "version": "1.1.1"},
    "dry_run": None,
}
