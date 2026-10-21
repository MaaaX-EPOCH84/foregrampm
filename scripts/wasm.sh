#!/usr/bin/env bash
# ForegramPM — checks that the product crates compile for WebAssembly.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage: scripts/wasm.sh
#
# Compiles `foregram-graph` and `foregram-render` (without the `png` feature) for
# `wasm32-unknown-unknown`, with no particular flag. Only compilation
# is held: nothing is run, no web API is exposed.
# The target is never installed here; if it is missing, the script says so.
#
# Exit codes: 0 compiles, 1 compilation fails, 2 tool or target missing.

set -euo pipefail

target=wasm32-unknown-unknown

fail() { echo "wasm: $1" >&2; exit 2; }

command -v cargo >/dev/null || fail "missing tool: cargo"
command -v rustup >/dev/null || fail "missing tool: rustup"

if ! rustup target list --installed | grep -qx "$target"; then
  fail "target $target is not installed: run \`rustup target add $target\`"
fi

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if cargo build --target "$target" -p foregram-graph -p foregram-render; then
  echo "wasm: foregram-graph and foregram-render build for $target"
else
  echo "wasm: build for $target failed" >&2
  exit 1
fi
