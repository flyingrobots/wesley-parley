# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Parley's message types in Python (`python/`, the package `wesley_parley`, flyingrobots/parley#121): stdlib-only `TypedDict` classes for Python 3.9 or later, rendered from the model through wesley-python's syntax API and printed by `ast.unparse`, with `python -m wesley_parley python [--model FILE] [--out FILE]`. The golden is compiled, imported and type-checked by `mypy --strict --python-version 3.9`; a missing required key, a wrong `event` and a `None` optional are proven type errors. CI runs ruff, mypy and the suite on Python 3.9 and 3.13, and the git hooks run them when `python/.venv` exists.
- Parley's vocabulary, interpreted once into a checked model (`vocab`, `model`), and its JSON Schema rendering (`json_schema`), with the `wesley-parley json-schema` and `wesley-parley model` commands. Parley's SDL at `c80771c` reproduces all 17 of its hand-written schemas as JSON values. Parley's message types in TypeScript (`typescript`, `wesley-parley typescript --out FILE`), type-checked by `tsc --strict`. CI and git hooks run `cargo fmt --check`, `clippy -D warnings` and the tests.
- The repository, its license, and the rules for how the target generates code (flyingrobots/parley#116, flyingrobots/wesley#882).
