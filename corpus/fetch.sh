#!/usr/bin/env bash
# ForegramPM — téléchargement et vérification de ce que le banc utilise :
# corpus publics et moteurs de référence.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage : corpus/fetch.sh [section...]   (toutes celles du manifeste par défaut)
#
# Le cache `corpus/cache/` appartient à ce script : il le remet à l'état
# du manifeste à chaque passage. Une archive déjà présente et conforme
# n'est pas retéléchargée ; son contenu est toujours réextrait.
#
# Codes de sortie : 0 conforme, 1 écart avec le manifeste, 2 appel,
# outil ou réseau.

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
manifest="$here/manifest.txt"
cache="$here/cache"

fail() { echo "fetch: $1" >&2; exit "${2:-2}"; }

for tool in curl sha256sum tar git find awk; do
  command -v "$tool" >/dev/null || fail "missing tool: $tool"
done
[ -r "$manifest" ] || fail "cannot read $manifest"

# Noms des sections, dans l'ordre du manifeste.
sections() {
  awk '/^\[[^]]+\]$/ { print substr($0, 2, length($0) - 2) }' "$manifest"
}

# Toutes les valeurs d'une clé dans une section, une par ligne.
fields() {
  awk -v s="[$1]" -v k="$2" '
    /^\[/ { in_s = ($0 == s); next }
    in_s && index($0, k " = ") == 1 { print substr($0, length(k) + 4) }
  ' "$manifest"
}

# Valeur d'une clé dans une section ; refuse si elle manque.
field() {
  local value
  value=$(awk -v s="[$1]" -v k="$2" '
    /^\[/ { in_s = ($0 == s); next }
    in_s && index($0, k " = ") == 1 { print substr($0, length(k) + 4); exit }
  ' "$manifest")
  [ -n "$value" ] || fail "manifest: [$1] has no '$2'"
  printf '%s\n' "$value"
}

# Compte les fichiers de graphe et compare au manifeste.
check_count() {
  local name=$1 dir=$2 pattern count found
  pattern=$(field "$name" pattern)
  count=$(field "$name" count)
  found=$(find "$dir" -path "$dir/.git" -prune -o -type f -name "$pattern" -print | wc -l)
  [ "$found" -eq "$count" ] \
    || fail "$name: $found files match '$pattern', manifest says $count" 1
  echo "$name: $found files match '$pattern'"
}

# Télécharge un fichier à moins qu'il soit déjà là avec la bonne empreinte.
download() {
  local name=$1 url=$2 sha=$3 file=$4 actual
  if [ -f "$file" ] && [ "$(sha256sum <"$file" | cut -d' ' -f1)" = "$sha" ]; then
    echo "$name: $(basename "$file") already present"
    return
  fi
  echo "$name: downloading $url"
  curl --fail --silent --show-error --location --output "$file.part" "$url" \
    || fail "$name: download failed"
  actual=$(sha256sum <"$file.part" | cut -d' ' -f1)
  if [ "$actual" != "$sha" ]; then
    rm -f "$file.part"
    fail "$name: SHA-256 is $actual, manifest says $sha" 1
  fi
  mv "$file.part" "$file"
}

fetch_archive() {
  local name=$1 url sha file dir
  url=$(field "$name" url)
  sha=$(field "$name" sha256)
  file="$cache/$name.tgz"
  dir="$cache/$name"

  download "$name" "$url" "$sha" "$file"
  echo "$name: SHA-256 matches"

  # L'archive a un seul répertoire racine, retiré à l'extraction.
  rm -rf "$dir"
  mkdir -p "$dir"
  tar -xzf "$file" -C "$dir" --strip-components=1 \
    || fail "$name: cannot extract $file" 1
  check_count "$name" "$dir"
}

fetch_git() {
  local name=$1 url commit dir head
  url=$(field "$name" url)
  commit=$(field "$name" commit)
  dir="$cache/$name"

  if [ ! -d "$dir/.git" ]; then
    rm -rf "$dir"
    git init --quiet "$dir"
    git -C "$dir" remote add origin "$url"
  fi
  if ! git -C "$dir" cat-file -e "$commit^{commit}" 2>/dev/null; then
    echo "$name: fetching $commit from $url"
    git -C "$dir" fetch --quiet --depth 1 origin "$commit" \
      || fail "$name: fetch failed"
  else
    echo "$name: commit already present"
  fi
  # Toute retouche locale est effacée : le cache suit le manifeste.
  if git -C "$dir" rev-parse --verify --quiet HEAD >/dev/null \
    && [ -n "$(git -C "$dir" status --porcelain --untracked-files=all)" ]; then
    echo "$name: local changes discarded"
  fi
  git -C "$dir" -c advice.detachedHead=false checkout --quiet --force --detach "$commit"
  git -C "$dir" clean --quiet -d --force -x

  head=$(git -C "$dir" rev-parse HEAD)
  [ "$head" = "$commit" ] || fail "$name: HEAD is $head, manifest says $commit" 1
  [ -z "$(git -C "$dir" status --porcelain --untracked-files=all)" ] \
    || fail "$name: working tree differs from $commit" 1
  echo "$name: commit matches"
  check_count "$name" "$dir"
}

# Un paquet Debian dépaqueté dans le cache, sans rien installer : Graphviz.
# Les greffons dont une bibliothèque manque sur le poste sont retirés, puis
# `dot -c` écrit la table des greffons à côté de l'exécutable.
fetch_deb() {
  local name=$1 url sha version dir file plugin found
  url=$(field "$name" url)
  sha=$(field "$name" sha256)
  version=$(field "$name" version)
  dir="$cache/$name"
  file="$cache/$name.deb"
  command -v dpkg >/dev/null || fail "missing tool: dpkg"
  command -v ldd >/dev/null || fail "missing tool: ldd"
  download "$name" "$url" "$sha" "$file"
  echo "$name: SHA-256 matches"
  rm -rf "$dir"
  mkdir -p "$dir"
  dpkg -x "$file" "$dir" || fail "$name: cannot unpack $file" 1
  for plugin in "$dir"/usr/lib/graphviz/libgvplugin_*.so*; do
    if LD_LIBRARY_PATH="$dir/usr/lib" ldd "$plugin" 2>/dev/null | grep -q 'not found'; then
      echo "$name: plugin $(basename "$plugin") left out, a library is missing"
      rm -f "$plugin"
    fi
  done
  LD_LIBRARY_PATH="$dir/usr/lib" "$dir/usr/bin/dot" -c 2>/dev/null \
    || fail "$name: dot -c failed" 1
  found=$(LD_LIBRARY_PATH="$dir/usr/lib" "$dir/usr/bin/dot" -V 2>&1 | awk '{ for (i = 1; i < NF; i++) if ($i == "version") print $(i + 1) }')
  [ "$found" = "$version" ] || fail "$name: dot is version $found, manifest says $version" 1
  echo "$name: dot $found ready"
}

# Des jars de Maven Central, et le pilote Java du banc compilé contre eux : ELK.
# Tout jar que le manifeste ne cite pas est retiré du cache.
fetch_maven() {
  local name=$1 version dir url sha file keep k found
  version=$(field "$name" version)
  dir="$cache/$name"
  command -v javac >/dev/null || fail "missing tool: javac (a JDK is needed to compile the ELK driver)"
  command -v java >/dev/null || fail "missing tool: java"
  mkdir -p "$dir/jars"
  keep=()
  while read -r url sha; do
    if [ -z "$url" ] || [ -z "$sha" ]; then
      fail "manifest: [$name] has a jar line without its SHA-256"
    fi
    file="$dir/jars/$(basename "$url")"
    download "$name" "$url" "$sha" "$file"
    keep+=("$file")
  done < <(fields "$name" jar)
  [ "${#keep[@]}" -gt 0 ] || fail "manifest: [$name] has no jar"
  for file in "$dir"/jars/*; do
    found=no
    for k in "${keep[@]}"; do [ "$k" = "$file" ] && found=yes; done
    [ "$found" = yes ] || { echo "$name: $(basename "$file") removed, not in the manifest"; rm -f "$file"; }
  done
  ls "$dir/jars/org.eclipse.elk.core-$version.jar" >/dev/null 2>&1 \
    || fail "$name: no org.eclipse.elk.core-$version.jar among the jars, manifest says version $version" 1
  echo "$name: ${#keep[@]} jars match"
  rm -rf "$dir/classes"
  mkdir -p "$dir/classes"
  javac -Xlint:all -Werror -cp "$(printf '%s:' "${keep[@]}")" -d "$dir/classes" \
    "$here/../crates/foregram-bench/java/ElkDriver.java" \
    || fail "$name: cannot compile the driver" 1
  echo "$name: driver compiled, ELK $version ready"
}

if [ "$#" -gt 0 ]; then
  names=("$@")
else
  mapfile -t names < <(sections)
fi

mkdir -p "$cache"
for name in "${names[@]}"; do
  sections | grep -qxF -- "$name" || fail "unknown section: $name"
  source=$(field "$name" source)
  case $source in
    archive) fetch_archive "$name" ;;
    git) fetch_git "$name" ;;
    deb) fetch_deb "$name" ;;
    maven) fetch_maven "$name" ;;
    *) fail "manifest: [$name] has an unknown source" ;;
  esac
done
