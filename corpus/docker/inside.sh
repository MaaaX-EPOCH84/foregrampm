#!/usr/bin/env bash
# ForegramPM — ce qui tourne dans le conteneur du déterminisme : copie des
# sources, cache remis au manifeste, compilation, planche, empreinte.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Attend /src (le dépôt, en lecture seule), /out (où écrire) et, s'il est
# monté, /hostcache (un cache déjà téléchargé, copié pour épargner le réseau).
# Écrit dans /out : env.txt, board.log, board/ et sums.txt.

set -euo pipefail

work=$(mktemp -d)
export HOME="$work/home"
mkdir -p "$HOME"

(cd /src && tar cf - --exclude=./target --exclude=./corpus/cache \
  --exclude=./corpus/board --exclude=./corpus/layouts --exclude=./corpus/local .) \
  | (cd "$work" && tar xf -)
if [ -d /hostcache ]; then
  mkdir -p "$work/corpus/cache"
  cp -r /hostcache/. "$work/corpus/cache/"
fi

cd "$work"
corpus/fetch.sh
cargo build --release -p foregram-bench
corpus/fingerprint.sh env > /out/env.txt
status=0
./target/release/foregram-bench board corpus/cache /out/board > /out/board.log 2>&1 || status=$?
echo "board exit $status" >> /out/board.log
corpus/fingerprint.sh sums /out/board > /out/sums.txt
