#!/usr/bin/env bash
# ForegramPM — what runs in the determinism container: copy of the
# sources, cache put back to the manifest, build, board, fingerprint.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Expects /src (the repository, read-only), /out (where to write) and, if
# mounted, /hostcache (an already downloaded cache, copied to spare the network).
# Writes to /out: env.txt, board.log, board/ and sums.txt.

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
