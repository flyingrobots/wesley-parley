#!/usr/bin/env bash
# Stages a hermetic Docker build context for the Parley end-to-end check.
#
# Every source goes in by copy, from committed git objects in local checkouts: no working tree is
# read, no network is used, and no git remote URL enters the context. Parley goes in as a git
# bundle (its tests read its history); every other repository as a plain tree.
#
#   e2e/parley/stage.sh <context-dir>
#
# Environment overrides (defaults in brackets):
#   GIT_ROOT        directory holding the checkouts [~/git]
#   PARLEY_REF      [origin/main]   WESLEY_REF     [v0.3.0-alpha.4]
#   TARGETS_REF     ref used for every wesley-* target repository [origin/main]
set -euo pipefail

ctx="${1:?usage: stage.sh <context-dir>}"
git_root="${GIT_ROOT:-$HOME/git}"
parley_ref="${PARLEY_REF:-origin/main}"
wesley_ref="${WESLEY_REF:-v0.3.0-alpha.4}"
targets_ref="${TARGETS_REF:-origin/main}"
here="$(cd "$(dirname "$0")" && pwd)"

if [ -e "$ctx" ] && [ -n "$(ls -A "$ctx" 2>/dev/null)" ]; then
  echo "error: $ctx exists and is not empty; give a new directory" >&2
  exit 2
fi
mkdir -p "$ctx/src"

manifest="$ctx/MANIFEST.tsv"
printf 'repository\tref\tcommit\tform\n' > "$manifest"

commit_of() { git -C "$1" rev-parse --verify --quiet "$2^{commit}"; }

# Parley: a bundle of main's full history, so its release-eligibility tests can read it.
parley="$git_root/parley"
parley_commit="$(commit_of "$parley" "$parley_ref")" || { echo "error: $parley_ref not found in $parley" >&2; exit 1; }
# A bundle needs ref names; bundle the ref plus every tag, then check it holds the commit.
git -C "$parley" bundle create -q "$ctx/src/parley.bundle" "$parley_ref" --tags
git -C "$parley" bundle list-heads "$ctx/src/parley.bundle" | grep -q "^$parley_commit " || { echo "error: the bundle does not hold $parley_commit" >&2; exit 1; }
printf 'parley\t%s\t%s\tbundle\n' "$parley_ref" "$parley_commit" >> "$manifest"

# Every other repository: the committed tree only.
stage_tree() {
  local repo="$1" ref="$2" commit
  commit="$(commit_of "$git_root/$repo" "$ref")" || { echo "error: $ref not found in $git_root/$repo" >&2; exit 1; }
  mkdir -p "$ctx/src/$repo"
  git -C "$git_root/$repo" archive --format=tar "$commit" | tar -x -C "$ctx/src/$repo"
  printf '%s\t%s\t%s\ttree\n' "$repo" "$ref" "$commit" >> "$manifest"
}
stage_tree wesley "$wesley_ref"
for repo in wesley-parley wesley-json-schema wesley-typescript wesley-rust wesley-python; do
  stage_tree "$repo" "$targets_ref"
done

# wesley-parley pins the generic targets by git URL. In the staged copy each becomes a path
# dependency on the staged tree, and the lockfile's git sources are dropped (a package without a
# source is a path package), so no URL is ever resolved or fetched.
wp="$ctx/src/wesley-parley"
# perl -pi edits in place and leaves no backup file behind (scratch files under /tmp are never
# deleted by this script).
perl -pi -e 's#^wesley-json-schema = \{ git = [^}]*\}#wesley-json-schema = { path = "../../../wesley-json-schema/crates/wesley-json-schema" }#; s#^wesley-typescript-syntax = \{ git = [^}]*\}#wesley-typescript-syntax = { path = "../../../wesley-typescript/crates/wesley-typescript-syntax" }#; s#^wesley-rust-syntax = \{ git = [^}]*\}#wesley-rust-syntax = { path = "../../../wesley-rust/crates/wesley-rust-syntax" }#' "$wp/crates/wesley-parley/Cargo.toml"
perl -ni -e 'print unless /^source = "git\+https:\/\/github\.com\/flyingrobots\/wesley-/' "$wp/Cargo.lock"
perl -ni -e 'print unless /^wesley-python @ git\+/' "$wp/python/requirements-dev.txt"

cp "$here/Dockerfile" "$here/run.sh" "$ctx/"
if grep -rIlE --exclude=MANIFEST.tsv -e 'git\+(https|ssh)://' -e '\{ *git *= *"' -e 'url = (https|git@)' "$ctx/src" >/dev/null 2>&1; then
  grep -rIlE --exclude=MANIFEST.tsv -e 'git\+(https|ssh)://' -e '\{ *git *= *"' -e 'url = (https|git@)' "$ctx/src" >&2
  echo "error: a git remote URL reached the build context" >&2
  exit 1
fi
echo "staged $(($(wc -l < "$manifest") - 1)) repositories into $ctx"
column -t -s $'\t' "$manifest"
