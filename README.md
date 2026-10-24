# ForegramPM

Project management in text files, driven by an AI: UML and Merise
diagrams written as text and rendered as images for the documentation.

The name: *foreman* + *diagram*, and *PM* for *project management*.

The project is being scoped — see `docs/genese.md` (in French).

The project lives on GitHub: https://github.com/MaaaX-EPOCH84/foregrampm.
Issues and pull requests go there; any other copy is only a mirror.

## Layout benchmark

The layout engine is judged on a benchmark. Its exchange format — the
graph an engine receives, the layout it returns — is described in
[`docs/exchange-format.md`](docs/exchange-format.md).

`foregram` is the product command line: it validates, rewrites in
canonical form, measures and draws files of that format — and checks and
rewrites the source file of a diagram, `.fg`, described in
[`docs/source-format.md`](docs/source-format.md) — and with
`--json` writes its results and refusals as JSON, one object per line;
`--lang fr` says them in French, English being the default.

```
cargo test
cargo run -p foregram -- validate <graph.json> [<layout.json>...]
cargo run -p foregram -- check <file.fg>...
cargo run -p foregram -- canon <file.json|file.fg>
cargo run -p foregram -- metrics <graph.json> <layout.json>...
cargo run -p foregram -- render <graph.json> <layout.json> <out.svg|out.png> [--scale <n>]
```

`foregram-bench` converts the third-party formats and drives the reference
engines; a converted graph goes to `foregram` through the standard input:

```
cargo run -p foregram-bench -- graphml g.graphml | cargo run -p foregram -- validate -
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

## Library

Two product crates, with no I/O and no dependency outside the `png`
feature: `foregram-graph` (graph, layout, strict JSON reader, validation,
canonical form, metrics) and `foregram-render` (SVG, and PNG behind the
`png` feature). Their public surface is frozen from **version 0.1.0** on:
each crate documents it at the top of its `src/lib.rs`, `cargo doc` renders
it, and a test (`tests/api.rs`) lists every exported name, so that a new
`pub` is a decision. One example per crate shows the whole path, bytes in,
values out:

```
cargo run -p foregram-graph --example measure
cargo run -p foregram-render --example draw > components.svg
```

## WebAssembly

`scripts/wasm.sh` checks that `foregram-graph` and `foregram-render` (without
the `png` feature) build for `wasm32-unknown-unknown` with no special flag.
Only the build is held: nothing is run and no web API is exposed. The target
is not installed by the script; if it is missing the script says so (exit 2):

```
rustup target add wasm32-unknown-unknown
scripts/wasm.sh
```

## Checking the project

`scripts/check.sh` runs every check and prints one line per check: rustfmt,
Clippy (default and all features, warnings denied), the tests (default and all
features), ShellCheck on the tracked shell scripts, the WebAssembly build, and
the copyright and SPDX header of every tracked `.rs`, `.sh` and `.java` file.
It exits 0 when all pass and 1 otherwise; a missing tool or target counts as a
failure.

```
scripts/check.sh
```

## Licence

GPL v3 or later — see [LICENSE](LICENSE).

Maxime MARCONATO (MaaaX / EPOCH84)
