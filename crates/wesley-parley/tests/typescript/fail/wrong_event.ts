import type { End } from "../../golden/messages.generated";

// `event` is the literal "end".
export const end: End = {
  event: "run", t: 1, seq: 4, run: "r", status: 0, receipt: "/r.jsonl", redacted: false,
  _schema: { id: "parley.end", version: "1.0.0" },
};
