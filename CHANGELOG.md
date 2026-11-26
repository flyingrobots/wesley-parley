# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Parley's vocabulary, interpreted once into a checked model (`vocab`, `model`), and its JSON Schema rendering (`json_schema`), with the `wesley-parley json-schema` and `wesley-parley model` commands. Parley's SDL at `c80771c` reproduces all 17 of its hand-written schemas as JSON values. CI and git hooks run `cargo fmt --check`, `clippy -D warnings` and the tests.
- The repository, its license, and the rules for how the target generates code (flyingrobots/parley#116, flyingrobots/wesley#882).
