import type { End, Line, RunBootstrap } from "../../golden/messages.generated";

const end: End = {
  event: "end", t: 1795352899890, seq: 4, run: "r", status: 0, receipt: "/r.jsonl", redacted: false,
  _schema: { id: "parley.end", version: "1.0.0" },
};

// An optional field may be omitted.
const boot: RunBootstrap = {
  event: "run", t: 1, seq: 0, tool: "count", v: "0.1.0", run: "r", paths: { receipt: "/r.jsonl" },
  _schema: { id: "parley.run.bootstrap", version: "1.1.1" },
};

// Line narrows on `event`.
export function status(line: Line): number | undefined {
  if (line.event === "end") {
    return line.status;
  }
  return undefined;
}

export const lines: Line[] = [end, boot];
