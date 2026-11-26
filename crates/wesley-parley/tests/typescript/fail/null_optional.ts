import type { RunBootstrap } from "../../golden/messages.generated";

// An optional field is absent when unset, never null.
export const boot: RunBootstrap = {
  event: "run", t: 1, seq: 0, tool: "count", v: "0.1.0", run: "r", paths: { receipt: "/r.jsonl" },
  _schema: { id: "parley.run.bootstrap", version: "1.1.1" }, dry_run: null,
};
