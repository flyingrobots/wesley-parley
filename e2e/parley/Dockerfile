# syntax=docker/dockerfile:1.7
#
# Hermetic end-to-end check: can Parley generate its schemas and bindings with the pinned Wesley
# and this wesley-parley? Built from a context staged by stage.sh (sources copied in, no git
# remote URL). Only the dependency-fetch steps below have network access, each resolving exactly
# what a lockfile pins; every build and test step runs with --network=none.

FROM ghcr.io/astral-sh/uv@sha256:a3ea1357342c1a4bf9a0c53a3deb1b55379aefa037338d8113d5771efa01774f AS uv
FROM oven/bun@sha256:ec06c3b6cea04192ae6770c434f668ca41d343ad19fa6472216c7b48be39c598 AS bun
FROM node@sha256:be979e5523c76fa4d5ca4ebd3099078f220aa47ec39c48ad5b5df9314debd6eb AS node

FROM rust@sha256:75d26c23c24cc6ea1cdabf0714a6bff8fd7bf3248716043fd7ae86dac7f41b94 AS e2e
COPY --from=uv /uv /usr/local/bin/uv
COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun
# Node and npm: Parley's release-gate tests run `npm pack`, TypeScript's launcher needs `node`, and
# wesley-parley's type-check tests run `npx`.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
 && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx
# The tests run as an ordinary user: several prove a read-only or unwritable location is handled,
# which root, able to write anywhere, would never see.
RUN useradd --create-home --uid 1000 e2e
RUN ln -s /usr/local/bin/bun /usr/local/bin/bunx \
 && apt-get update && apt-get install -y --no-install-recommends jq && rm -rf /var/lib/apt/lists/*
ENV UV_PYTHON_INSTALL_DIR=/opt/python CARGO_TERM_COLOR=never CI=1

COPY src/ /src/
COPY MANIFEST.tsv run.sh /e2e/

# Parley, cloned from the bundle with its history; the clone's remote is removed.
RUN commit="$(awk -F'\t' '$1 == "parley" { print $3 }' /e2e/MANIFEST.tsv)" \
 && git clone -q --no-checkout /src/parley.bundle /work/parley \
 && git -C /work/parley checkout -q -B main "$commit" \
 && git -C /work/parley remote remove origin \
 && git -C /work/parley config user.email e2e@localhost && git -C /work/parley config user.name e2e

# --- Dependency fetch: the only steps with network, each pinned by a lockfile or exact version ---
RUN uv python install 3.9 3.13
RUN cd /src/wesley && cargo fetch --locked
RUN cd /src/wesley-parley && cargo fetch --locked
RUN cd /work/parley && bun install --frozen-lockfile
RUN uv venv -q --python 3.9 /venv/py39 \
 && uv pip install -q --python /venv/py39/bin/python setuptools wheel "mypy==1.19.1"
# typescript@5 for wesley-parley's `npx -p typescript@5 tsc` tests, cached for the e2e user.
RUN su e2e -c "npx --yes -p typescript@5 tsc --version"

# --- Hermetic from here: no network ---
RUN --network=none cd /src/wesley && cargo build --release --offline --locked -p wesley-cli \
 && install -m 0755 target/release/wesley /usr/local/bin/wesley
RUN --network=none cd /src/wesley-parley && cargo build --release --offline --locked \
 && install -m 0755 target/release/wesley-parley /usr/local/bin/wesley-parley
# Its test binaries too, two links at a time: each links all of swc, and ten at once exceed a
# 4 GB Docker VM (the linker is killed). The run then only executes them.
RUN --network=none cd /src/wesley-parley && cargo test --offline --locked --no-run -j 2
RUN --network=none uv pip install -q --offline --no-deps --no-build-isolation \
      --python /venv/py39/bin/python /src/wesley-python /src/wesley-parley/python

# Hand the build products to the test user; npm may only use its cache from here on.
RUN chown -R e2e:e2e /work /src /venv /usr/local/cargo /e2e
ENV PATH="/venv/py39/bin:${PATH}" npm_config_offline=true
USER e2e
WORKDIR /work/parley
ENTRYPOINT ["/e2e/run.sh"]
