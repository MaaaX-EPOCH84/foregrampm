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

## Licence

GPL v3 or later — see [LICENSE](LICENSE).

Maxime MARCONATO (MaaaX / EPOCH84)
