#!/usr/bin/env bash
# ForegramPM — runs the board in the second environment (Docker) and reports
# its fingerprint, to compare with that of this machine by fingerprint.sh.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage: corpus/docker/run.sh <out>
#
# Builds the image, then writes to <out>: env.txt, board.log, board/ and
# sums.txt, as the current user. This machine's cache is lent to the
# container read-only if it exists; fetch.sh rechecks it there.
# Exit code: Docker's, 2 for an incorrect call.

set -euo pipefail

[ $# -eq 1 ] || { echo "usage: run.sh <out>" >&2; exit 2; }
command -v docker >/dev/null || { echo "run.sh: missing tool: docker" >&2; exit 2; }

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=$(mkdir -p "$1" && cd "$1" && pwd)

docker build -t foregram-determinism "$here"
mounts=(-v "$root:/src:ro" -v "$out:/out")
[ -d "$root/corpus/cache" ] && mounts+=(-v "$root/corpus/cache:/hostcache:ro")
docker run --rm --user "$(id -u):$(id -g)" "${mounts[@]}" foregram-determinism
