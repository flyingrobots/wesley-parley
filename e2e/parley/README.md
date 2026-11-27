# Parley end-to-end check

Does Parley actually build with the pinned Wesley and this wesley-parley? This check answers that in a hermetic Docker image, before anything is released.

## What it proves

Run against a clone of Parley with the network disabled (`docker run --network none`):

1. Parley's own gate passes unchanged (`bun test`, `bun run check`, `bun run typecheck`), with the Wesley version Parley pins on `PATH`.
2. Parley's SDL, lowered by that Wesley, renders all 17 message schemas, each equal to Parley's hand-written schema as a JSON value.
3. Every message version checks against `schemas/released/` (`wesley-parley versions`).
4. The TypeScript module type-checks inside both Parley packages under Parley's own compiler and settings; a valid use passes and every invalid use is a type error.
5. The Python module is standard-library only, imports on Python 3.9 and passes `mypy --strict`; spliced into `parley_emit.py`, the writer's own suite still passes.
6. The Rust module equals wesley-parley's compiled, round-tripped golden, and wesley-parley's suite passes offline.
7. With the generated schemas swapped in as the source, the only refusal is Parley's rule that a top-level schema is byte-identical to its released snapshot. Rule (A) of flyingrobots/parley#116 replaces that rule with JSON-value identity (spec change flyingrobots/parley#117), so this step reports `EXPECTED` until that lands, and fails on any other refusal.

## How it stays hermetic

- `stage.sh` copies every source into the build context from committed git objects in local checkouts: Parley as a git bundle (its tests read its history), every other repository as a plain tree, each at a commit recorded in `MANIFEST.tsv`. No working tree is read.
- wesley-parley's git dependencies are rewritten to path dependencies on the staged trees, and their lockfile sources dropped, so no git remote URL enters the context; `stage.sh` refuses to finish if one does.
- Base images are pinned by digest. Only the dependency-fetch steps of the build have network, each resolving exactly what a lockfile or exact version pins; every build and test step runs with `--network=none`, and the check itself runs in a container with no network, as an ordinary user (several of Parley's tests prove an unwritable location is handled, which root never sees).

## Run it

```sh
e2e/parley/stage.sh /tmp/wesley-parley-e2e
docker build -t wesley-parley-e2e /tmp/wesley-parley-e2e
mkdir -p /tmp/e2e-out && chmod 777 /tmp/e2e-out
docker run --rm --network none -v /tmp/e2e-out:/out wesley-parley-e2e
```

`GIT_ROOT` (default `~/git`) names the directory holding the checkouts; `PARLEY_REF`, `WESLEY_REF` and `TARGETS_REF` choose the commits (defaults: `origin/main`, `v0.3.0-alpha.4`, `origin/main`). The generated files and logs land in the mounted `/out`. A 4 GB Docker VM is enough; the image links wesley-parley's test binaries two at a time for that reason.
