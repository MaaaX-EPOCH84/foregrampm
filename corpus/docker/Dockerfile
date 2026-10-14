# ForegramPM — second environnement pour la vérification du déterminisme du
# banc : autre distribution, autre libc, autre JVM, autre rustc, même CPU.
# Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Usage : corpus/docker/run.sh <out>   (voir ce script)

FROM debian:trixie-slim

# Ce que fetch.sh, cargo et les moteurs demandent ; libltdl7, zlib1g et
# libexpat1 sont les seules bibliothèques système du dot d'Ubuntu.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git tar gawk dpkg findutils coreutils \
    build-essential openjdk-21-jdk-headless libltdl7 zlib1g libexpat1 \
    && rm -rf /var/lib/apt/lists/*

# Rust stable du jour, hors de /root pour tourner sous l'utilisateur de l'hôte.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
    && chmod -R a+rwX /usr/local/rustup /usr/local/cargo

COPY inside.sh /usr/local/bin/inside.sh
CMD ["bash", "/usr/local/bin/inside.sh"]
