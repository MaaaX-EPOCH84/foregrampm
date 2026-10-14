# ForegramPM

Project management in text files, driven by an AI: UML and Merise
diagrams written as text and rendered as images for the documentation.

The name: *foreman* + *diagram*, and *PM* for *project management*.

The project is being scoped — see `docs/genese.md` (in French).

The project lives on GitLab: https://gitlab.com/MaaaX-EPOCH84/foregrampm.
Issues and merge requests go there; any other copy is only a mirror.

## Layout benchmark

The layout engine is judged on a benchmark. Its exchange format — the
graph an engine receives, the layout it returns — is described in
[`docs/exchange-format.md`](docs/exchange-format.md).

```
cargo test
cargo run -p foregram-bench -- validate <graph.json> [<layout.json>...]
```

The public corpora — Rome, North and elk-models — are not versioned.
`corpus/fetch.sh` downloads them into `corpus/cache/` (ignored by Git)
and checks them against [`corpus/manifest.txt`](corpus/manifest.txt),
which records each address, version, SHA-256 or commit, file count and
licence. Running it again downloads nothing and checks everything; it
needs `curl`, `git`, `tar` and `sha256sum`.

```
corpus/fetch.sh [rome] [north] [elk-models]
```

Rome and North are GraphML; the benchmark converts them in memory. The
check below converts all 12,811 graphs and compares each with the
`Graph.log` of its archive:

```
cargo run --release -p foregram-bench -- check-corpus corpus/cache
cargo run -p foregram-bench -- graphml <file.graphml>
```

`corpus/fingerprint.sh` takes the SHA-256 of everything the bench writes
and compares two runs; `corpus/docker/run.sh` repeats a full run in a
Debian container to compare environments. Both are described in the
section "Determinism" of the format document. The figures of both
reference engines, recorded as the bar the own layout engine has to beat,
are under [`corpus/bar/`](corpus/bar/README.md).

## Licence

GPL v3 or later — see [LICENSE](LICENSE).

Maxime MARCONATO (MaaaX / EPOCH84)
