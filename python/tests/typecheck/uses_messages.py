"""Uses of the golden module that mypy --strict must accept."""

from __future__ import annotations

from typing import List

from messages import End, Line, QueryWindow, ReadQuery, Redaction, RunBootstrap

end: End = {
    "event": "end",
    "t": 1795352899890,
    "seq": 4,
    "run": "r",
    "status": 0,
    "receipt": "/r.jsonl",
    "redacted": False,
    "_schema": {"id": "parley.end", "version": "1.0.0"},
}

# An optional key (dry_run) omitted type-checks.
boot: RunBootstrap = {
    "event": "run",
    "t": 1,
    "seq": 0,
    "tool": "count",
    "v": "0.1.0",
    "run": "r",
    "paths": {"receipt": "/r.jsonl"},
    "_schema": {"id": "parley.run.bootstrap", "version": "1.1.1"},
}
boot["dry_run"] = True

# Keyword keys are typed: a query window's ``from`` and a redaction's ``in`` read as ``int``.
window: QueryWindow = {"from": 1, "to": 9}
start: int = window["from"]


def first_seq(query: ReadQuery) -> int:
    """Read the first sequence number of a query's window."""
    return query["window"]["from"]


def affected(redaction: Redaction) -> int:
    """Read the sequence number a redaction affects."""
    return redaction["in"]


# Every message is a Line.
lines: List[Line] = [end, boot]
