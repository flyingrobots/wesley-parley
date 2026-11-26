"""A wrong ``event`` literal must fail mypy."""

from __future__ import annotations

from messages import End

end: End = {
    "event": "run",
    "t": 1,
    "seq": 4,
    "run": "r",
    "status": 0,
    "receipt": "/r.jsonl",
    "redacted": False,
    "_schema": {"id": "parley.end", "version": "1.0.0"},
}
