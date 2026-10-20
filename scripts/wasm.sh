#!/usr/bin/env bash
# ForegramPM — vérifie que les crates de produit se compilent pour WebAssembly.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage : scripts/wasm.sh
#
# Compile `foregram-graph` et `foregram-render` (sans la feature `png`) pour
# `wasm32-unknown-unknown`, sans drapeau particulier. Seule la compilation
# est tenue : rien n'est exécuté, aucune API n'est exposée pour le web.
# La cible n'est jamais installée ici ; si elle manque, le script le dit.
#
# Codes de sortie : 0 compile, 1 la compilation échoue, 2 outil ou cible absents.

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
