# wesley-parley

A [Wesley](https://github.com/flyingrobots/wesley) target that emits Parley bindings for tool authors: a typed emit function and a typed reader for each message a tool declares, the `_schema` stamp filled in, and the message's JSON Schema in Parley's envelope.

Wesley compiles GraphQL SDL into an intermediate representation (IR), and does nothing else: it attaches no meaning to any directive and runs no targets. Every output language is its own repository, a standalone program and library that reads the IR (`wesley schema lower --json`). This one owns **Parley's vocabulary**: the `@parley_*` directives Parley declares in its own SDL mean something here and nowhere else. It renders through the generic targets ([wesley-json-schema](https://github.com/flyingrobots/wesley-json-schema), [wesley-rust](https://github.com/flyingrobots/wesley-rust), [wesley-typescript](https://github.com/flyingrobots/wesley-typescript), [wesley-python](https://github.com/flyingrobots/wesley-python)). The decisions are recorded in flyingrobots/parley#116 and flyingrobots/wesley#882.

## Status

Not started. This repository holds the plan and its rules; the first code waits on the versioned IR contract (flyingrobots/wesley#881).

## Where it comes from

Parley ([flyingrobots/parley](https://github.com/flyingrobots/parley)) is a specification of JSON Lines messages for agent-facing command-line tools. A tool author today writes each result message's JSON Schema by hand and repeats its id and version as strings, and the emitted payload is untyped. This target generates those from the tool's SDL, and enforces Parley's per-message versioning (spec §4.5) against the released snapshots. Parley stays the definition of the protocol: this target conforms to the spec and never defines it.

## How it generates code

The output is built as a syntax tree of the target language and printed by that language's printer. There is no string templating and no regex anywhere in the generator.

| Concern | Choice |
| --- | --- |
| Written in | Rust (stable), plus a Python module for the Python bindings |
| Syntax tree | each language's, through the generic targets: `serde_json` values (wesley-json-schema), `syn` (wesley-rust), `oxc_ast` (wesley-typescript), stdlib `ast` (wesley-python) |
| Printer | each language's own: canonical `serde_json`, `prettyplease`, `oxc_codegen`, `ast.unparse` |
| Checked by | each language's toolchain on every golden file (JSON Schema meta-schema, `cargo check`, `tsc --strict`, `py_compile` and a type checker), Parley's 17 released schemas reproduced as JSON values, and a golden tool run through `@flyingrobots/parley-emit` whose output the Parley doctor accepts |

Rules every change keeps:

- **No code as strings.** A generated identifier, literal or comment enters the output as a node of the tree, never by concatenating source text. Escaping is the printer's job.
- **Golden output is checked by the real toolchain.** Every golden file is compiled or type-checked in CI by the toolchain named above, not only compared byte for byte.
- **Deterministic to the byte.** The same IR and options give the same bytes on every machine. CI runs the generator twice and compares.
- **The IR is the only input.** The target reads the published IR (flyingrobots/wesley#881) and never depends on Wesley's internals.

## License

Apache 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
