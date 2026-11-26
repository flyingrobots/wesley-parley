import type { End } from "../../golden/messages.generated";

// `status` is required.
export const end: End = {
  event: "end", t: 1, seq: 4, run: "r", receipt: "/r.jsonl", redacted: false,
  _schema: { id: "parley.end", version: "1.0.0" },
};
