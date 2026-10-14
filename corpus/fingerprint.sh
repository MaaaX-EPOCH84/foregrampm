#!/usr/bin/env bash
# ForegramPM — empreinte d'une sortie du banc, pour prouver que deux passages
# donnent les mêmes octets : sur ce poste, puis d'un environnement à l'autre.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage : corpus/fingerprint.sh env                   décrit l'environnement
#         corpus/fingerprint.sh sums <dir>            SHA-256 de chaque fichier
#         corpus/fingerprint.sh digest <sums>         une empreinte par groupe
#         corpus/fingerprint.sh compare <sums> <sums> écarts, groupe par groupe
#
# `sums` écrit sur la sortie standard une ligne `<sha256>  <chemin>` par
# fichier régulier sous <dir>, chemin relatif, dans l'ordre des octets. Un
# groupe est fait des deux premiers composants d'un chemin (`layouts/dot`,
# `svg/elk`) ou du chemin entier s'il en a moins (`metrics.json`) : c'est le
# grain auquel un écart se lit, moteur par moteur.
#
# Codes de sortie : 0 identique ou fait, 1 écart entre deux empreintes,
# 2 appel ou outil.

set -euo pipefail

fail() { echo "fingerprint: $1" >&2; exit 2; }

for tool in find sha256sum sort awk; do
  command -v "$tool" >/dev/null || fail "missing tool: $tool"
done

# Première ligne de la sortie d'une commande, ou `-` si elle manque.
first_line() {
  if command -v "$1" >/dev/null; then
    "$@" 2>&1 | head -n 1
  else
    echo "-"
  fi
}

env_description() {
  local os="-"
  if [ -r /etc/os-release ]; then
    os=$(awk -F= '$1 == "PRETTY_NAME" { gsub(/"/, "", $2); print $2 }' /etc/os-release)
  fi
  echo "arch = $(uname -m)"
  echo "kernel = $(uname -s) $(uname -r)"
  echo "os = $os"
  echo "libc = $(first_line ldd --version)"
  echo "java = $(first_line java -version)"
  echo "rustc = $(first_line rustc --version)"
}

sums() {
  local dir="$1"
  [ -d "$dir" ] || fail "not a directory: $dir"
  (
    cd "$dir"
    find . -type f -printf '%P\0' | LC_ALL=C sort -z | xargs -0 -r sha256sum
  )
}

# Groupe d'un chemin : ses deux premiers composants.
group_of() {
  awk '{ n = split($2, p, "/"); print (n > 2 ? p[1] "/" p[2] : $2) }'
}

# Lignes d'un groupe, dans l'ordre du fichier.
lines_of() {
  awk -v g="$2" '{ n = split($2, p, "/"); if ((n > 2 ? p[1] "/" p[2] : $2) == g) print }' "$1"
}

# Une empreinte par groupe : SHA-256 des lignes du groupe, et leur nombre.
digest() {
  local sums="$1" g
  [ -r "$sums" ] || fail "cannot read $sums"
  while read -r g; do
    printf '%s  %6d files  %s\n' \
      "$(lines_of "$sums" "$g" | sha256sum | cut -d' ' -f1)" \
      "$(lines_of "$sums" "$g" | wc -l)" \
      "$g"
  done < <(group_of < "$sums" | awk '!seen[$0]++')
}

# `<chemin> <empreinte>` trié par chemin, pour `join`.
by_path() {
  awk '{ print $2 "\t" $1 }' "$1" | LC_ALL=C sort -t "$(printf '\t')" -k1,1
}

compare() {
  local a="$1" b="$2"
  [ -r "$a" ] || fail "cannot read $a"
  [ -r "$b" ] || fail "cannot read $b"
  LC_ALL=C join -t "$(printf '\t')" -a 1 -a 2 -e - -o 0,1.2,2.2 \
    <(by_path "$a") <(by_path "$b") |
  awk -F '\t' -v A="$a" -v B="$b" '
    {
      n = split($1, p, "/")
      g = (n > 2 ? p[1] "/" p[2] : $1)
      if (!(g in seen)) { seen[g] = 1; order[++count] = g }
      if ($2 == "-") { onlyb[g]++; what = "only in " B }
      else if ($3 == "-") { onlya[g]++; what = "only in " A }
      else if ($2 == $3) { same[g]++; next }
      else { diff[g]++; what = "differs" }
      if (++listed[g] <= 20) detail[g] = detail[g] "  " $1 ": " what "\n"
    }
    END {
      status = 0
      for (i = 1; i <= count; i++) {
        g = order[i]
        if (diff[g] + onlya[g] + onlyb[g] == 0) {
          printf "%-14s identical, %d files\n", g, same[g]
          continue
        }
        status = 1
        printf "%-14s %d identical, %d differ, %d only in %s, %d only in %s\n", \
          g, same[g] + 0, diff[g] + 0, onlya[g] + 0, A, onlyb[g] + 0, B
        printf "%s", detail[g]
        if (listed[g] > 20) printf "  ... %d more\n", listed[g] - 20
      }
      exit status
    }
  '
}

case "${1:-}" in
  env) [ $# -eq 1 ] || fail "usage: fingerprint.sh env"; env_description ;;
  sums) [ $# -eq 2 ] || fail "usage: fingerprint.sh sums <dir>"; sums "$2" ;;
  digest) [ $# -eq 2 ] || fail "usage: fingerprint.sh digest <sums>"; digest "$2" ;;
  compare) [ $# -eq 3 ] || fail "usage: fingerprint.sh compare <sums> <sums>"; compare "$2" "$3" ;;
  *) fail "usage: fingerprint.sh env | sums <dir> | digest <sums> | compare <sums> <sums>" ;;
esac
