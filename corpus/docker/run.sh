#!/usr/bin/env bash
# ForegramPM — lance la planche dans le second environnement (Docker) et en
# rapporte l'empreinte, à comparer à celle de ce poste par fingerprint.sh.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage : corpus/docker/run.sh <out>
#
# Construit l'image, puis écrit dans <out> : env.txt, board.log, board/ et
# sums.txt, sous l'utilisateur courant. Le cache de ce poste est prêté au
# conteneur en lecture seule s'il existe ; fetch.sh le revérifie là-bas.
# Code de sortie : celui de Docker, 2 pour un appel incorrect.

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
