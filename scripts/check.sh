#!/usr/bin/env bash
# ForegramPM — single project check: formatting, lints, tests, shell scripts,
# WebAssembly build and source headers.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage: scripts/check.sh
#
# Runs every check, even after a failure, then prints one line per check.
# A missing tool or target fails its check: a check that cannot run is not a
# success. The message catalogue needs no step of its own: the guards in
# `crates/foregram-graph/tests/messages.rs` (keys, codes, `fr.txt` against
# `en.txt`) run under `cargo test`.
#
# Exit codes: 0 every check passed, 1 at least one failed.

# The check functions are only called through `run`, which shellcheck cannot see.
# shellcheck disable=SC2317
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

copyright='Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)'
spdx='SPDX-License-Identifier: GPL-3.0-or-later'

names=()
results=()

# run <name> <command...>: run one check, record its outcome.
run() {
  local name=$1
  shift
  echo "== $name"
  if "$@"; then
    results+=("ok")
  else
    results+=("FAILED")
  fi
  names+=("$name")
}

# tracked <pattern...>: tracked files matching the pathspecs, NUL-separated.
tracked() { git ls-files -z -- "$@"; }

need() {
  command -v "$1" >/dev/null || { echo "check: missing tool: $1" >&2; return 1; }
}

shell_scripts() {
  need shellcheck || return 1
  local files=()
  while IFS= read -r -d '' f; do files+=("$f"); done < <(tracked '*.sh')
  [ "${#files[@]}" -gt 0 ] || { echo "check: no shell script found" >&2; return 1; }
  shellcheck "${files[@]}"
}

headers() {
  local bad=0 f
  while IFS= read -r -d '' f; do
    local head
    head=$(head -n 5 "$f")
    if [[ $head != *"$copyright"* ]] || [[ $head != *"$spdx"* ]]; then
      echo "check: missing or wrong header in the first 5 lines: $f" >&2
      bad=1
    fi
  done < <(tracked '*.rs' '*.sh' '*.java')
  return "$bad"
}

run "rustfmt" cargo fmt --check
run "clippy" cargo clippy --workspace --all-targets -- -D warnings
run "clippy, all features" cargo clippy --workspace --all-targets --all-features -- -D warnings
run "tests" cargo test --workspace
run "tests, all features" cargo test --workspace --all-features
run "shellcheck" shell_scripts
run "webassembly" scripts/wasm.sh
run "headers" headers

echo
status=0
for i in "${!names[@]}"; do
  printf '%-24s %s\n' "${names[$i]}" "${results[$i]}"
  [ "${results[$i]}" = "ok" ] || status=1
done
exit "$status"
