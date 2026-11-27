#!/usr/bin/env bash
# The end-to-end check, run inside the hermetic image with --network none, in a clone of Parley.
# Each step reports PASS or FAIL; the run continues so one pass shows every result, and exits 1 if
# any step failed. Generated files go under /out (mount it to keep them).
set -uo pipefail

out=/out
mkdir -p "$out"
results=()
failed=0

step() {
  local name="$1"
  shift
  echo
  echo "=== $name"
  "$@"
  case $? in
    0) results+=("PASS      $name") ;;
    3) results+=("EXPECTED  $name") ;;
    *) results+=("FAIL      $name"); failed=$((failed + 1)) ;;
  esac
}

sdl=schemas/parley.graphql
pin="$(jq -r .config.wesley package.json)"

versions() {
  echo "network: $(getent hosts example.com >/dev/null 2>&1 && echo AVAILABLE || echo none)"
  echo "wesley $(wesley --version) (Parley pins $pin)"
  wesley-parley --help | head -1
  python3 --version
  bun --version
  column -t -s $'\t' /e2e/MANIFEST.tsv
  [ "$(wesley --version)" = "$pin" ]
}

# 1. Parley's own gate, unchanged, with the pinned Wesley on PATH (CI=1: the SDL test may not skip).
parley_gate() { bun test && bun run check && bun run typecheck; }

lower() { wesley schema lower --schema "$sdl" --json > "$out/ir.json"; }

# 2. The generated schemas equal the hand-written ones as JSON values (rule (A)).
schemas_equal() {
  rm -rf "$out/schemas" && wesley-parley json-schema --ir "$out/ir.json" --out "$out/schemas" || return 1
  local n=0 bad=0 f id
  for f in schemas/*.schema.json; do
    id="$(basename "$f")"
    n=$((n + 1))
    if ! cmp -s <(jq -S . "$f") <(jq -S . "$out/schemas/$id"); then
      echo "differs: $id"
      bad=$((bad + 1))
    fi
  done
  echo "$((n - bad)) of $n schemas equal as JSON values; $(ls "$out/schemas" | wc -l) generated"
  [ "$bad" -eq 0 ] && [ "$n" -eq 17 ] && [ "$(ls "$out/schemas" | wc -l)" -eq "$n" ]
}

# 3. Every message version checks against the released snapshots (spec §4.5).
versions_check() { wesley-parley versions --ir "$out/ir.json" --released schemas/released; }

# 4. The generated schemas become the source. Validation cannot differ (step 2 proves every schema
#    equal as a JSON value, and validation reads only the value), so what this step finds is what
#    else in Parley observes bytes. Today that is the rule that a top-level schema is byte-identical
#    to its released snapshot, which rule (A) of flyingrobots/parley#116 replaces with JSON-value
#    identity (spec change flyingrobots/parley#117). The step passes if Parley accepts the swap, is
#    EXPECTED if that byte rule is the only refusal, and fails on anything else.
schemas_as_source() {
  cp "$out/schemas/"*.schema.json schemas/ || return 1
  bun run check > "$out/check.jsonl" 2>&1 && return 0
  local other
  other="$(grep '"event":"error"' "$out/check.jsonl" | grep -v 'two files claim .* with different bytes; a released version is immutable' || true)"
  if [ -z "$other" ] && grep -q 'with different bytes; a released version is immutable' "$out/check.jsonl"; then
    grep -o 'schemas/released/[^:]*' "$out/check.jsonl" | sort -u | sed 's/^/  byte rule: /'
    return 3
  fi
  echo "$other"
  return 1
}

# 5. TypeScript: the generated module type-checks in both packages under Parley's own compiler and
#    settings, and a valid use passes while invalid uses fail.
typescript() {
  wesley-parley typescript --ir "$out/ir.json" --out "$out/messages.generated.ts" || return 1
  cp "$out/messages.generated.ts" packages/parley-emit/src/messages.generated.ts
  cp "$out/messages.generated.ts" packages/parley-read/src/messages.generated.ts
  bun run typecheck || return 1
  # The fixtures import "../../golden/messages.generated", so mirror wesley-parley's tests/ layout, and
  # run Parley's compiler from there: with no tsconfig.json in the directory, files on the command
  # line are what it checks.
  local tsc="$PWD/node_modules/.bin/tsc" dir="$out/ts-uses" fixtures=/src/wesley-parley/crates/wesley-parley/tests/typescript
  rm -rf "$dir" && mkdir -p "$dir/golden" "$dir/typescript/pass" "$dir/typescript/fail"
  cp "$out/messages.generated.ts" "$dir/golden/"
  cp "$fixtures/pass/"*.ts "$dir/typescript/pass/"
  cp "$fixtures/fail/"*.ts "$dir/typescript/fail/"
  local flags=(--noEmit --strict --target es2022 --module es2022 --moduleResolution bundler)
  (cd "$dir" && "$tsc" "${flags[@]}" golden/messages.generated.ts typescript/pass/*.ts) || return 1
  echo "a valid use type-checks under Parley's TypeScript ($("$tsc" --version))"
  local f ok=0
  for f in "$dir/typescript/fail/"*.ts; do
    if (cd "$dir" && "$tsc" "${flags[@]}" golden/messages.generated.ts "typescript/fail/$(basename "$f")") >/dev/null 2>&1; then
      echo "expected a type error: $(basename "$f")"
      ok=1
    fi
  done
  [ "$ok" -eq 0 ] && echo "every invalid use is a type error ($(ls "$dir/typescript/fail" | wc -l))"
  return "$ok"
}

# 6. Python: the generated module is stdlib-only, imports on 3.9 and passes mypy --strict; spliced
#    into the vendorable writer, the writer's own suite still passes.
python_bindings() {
  wesley-parley model --ir "$out/ir.json" | python3 -m wesley_parley python --out "$out/messages.py" || return 1
  python3 -I -c "import ast, sys; tree = ast.parse(open('$out/messages.py').read()); mods = {n.module for n in ast.walk(tree) if isinstance(n, ast.ImportFrom)} | {a.name for n in ast.walk(tree) if isinstance(n, ast.Import) for a in n.names}; bad = mods - set(sys.stdlib_module_names if hasattr(sys, 'stdlib_module_names') else {'typing'}); print('imports:', sorted(mods)); sys.exit(1 if bad - {'typing'} else 0)" \
    && python3 -c "import importlib.util as u; s = u.spec_from_file_location('m', '$out/messages.py'); m = u.module_from_spec(s); s.loader.exec_module(m); print('imported', len([k for k in dir(m) if not k.startswith('_')]), 'names')" \
    && python3 -m mypy --strict --python-version 3.9 "$out/messages.py" || return 1
  local writer=packages/parley-emit-py/parley_emit.py
  python3 - "$writer" "$out/messages.py" <<'PY' || return 1
import sys
writer, generated = sys.argv[1], sys.argv[2]
text = open(writer).read()
block = "# BEGIN WESLEY\n" + open(generated).read() + "# END WESLEY\n"
marker = "# END SCHEMAS\n"
assert marker in text, "no END SCHEMAS marker"
open(writer, "w").write(text.replace(marker, marker + "\n" + block, 1))
PY
  (cd packages/parley-emit-py && python3 -m unittest discover -s tests >/dev/null 2>"$out/py-tests.log"; rc=$?; tail -3 "$out/py-tests.log"; exit "$rc") \
    && python3 -I "$writer" --vendor-info >/dev/null
}

# 7. Rust: the generated module for Parley's SDL is the one wesley-parley compiles and round-trips
#    in its own suite, which runs here offline.
rust_bindings() {
  wesley-parley rust --ir "$out/ir.json" --out "$out/messages.rs" || return 1
  if cmp -s "$out/messages.rs" /src/wesley-parley/crates/wesley-parley/tests/golden/messages.rs; then
    echo "messages.rs equals wesley-parley's compiled, round-tripped golden"
  else
    echo "note: Parley's SDL differs from wesley-parley's fixture; compiling the generated module"
  fi
  local rc=0
  (cd /src/wesley-parley && cargo test --offline --locked -j 2) > "$out/cargo-test.log" 2>&1 || rc=$?
  grep -E "^test result|FAILED|panicked|^error" "$out/cargo-test.log"
  return "$rc"
}

step "toolchain and pins" versions
step "Parley's own gate (bun test, check, typecheck)" parley_gate
step "lower Parley's SDL with the pinned Wesley" lower
step "17 generated schemas equal the hand-written ones as JSON values" schemas_equal
step "every message version checks against schemas/released" versions_check
step "TypeScript types type-check in both packages; invalid uses fail" typescript
step "Python TypedDicts: stdlib-only, mypy --strict on 3.9, spliced writer's suite passes" python_bindings
step "Rust types: wesley-parley's suite offline" rust_bindings
step "generated schemas as the source: only the byte-identity rule parley#117 replaces refuses them" schemas_as_source

echo
echo "=== Summary"
printf '%s\n' "${results[@]}"
[ "$failed" -eq 0 ] && echo "ALL PASSED" || echo "$failed step(s) failed"
exit $(( failed > 0 ))
