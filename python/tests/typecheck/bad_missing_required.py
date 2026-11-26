"""Omitting a required key (``status``) must fail mypy."""

from __future__ import annotations

from messages import End

end: End = {
    "event": "end",
    "t": 1,
    "seq": 4,
    "run": "r",
    "receipt": "/r.jsonl",
    "redacted": False,
    "_schema": {"id": "parley.end", "version": "1.0.0"},
}
