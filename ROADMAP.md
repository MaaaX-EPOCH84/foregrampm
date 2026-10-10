# Roadmap — ForegramPM

Status: ⬜ to do · 🔶 in progress · ✅ done · ❌ cancelled

| # | id | Stage | Content | Status |
|---|---|---|---|---|
| 1 | E1 | Corpus and exchange format | Exchange format with ports, nesting and curves; public and in-house corpora imported | 🔶 |
| 2 | E14 | Measurement | ELK and Graphviz as references, numeric metrics, neutral rendering, comparison board, bar to beat | ⬜ |
| 3 | E2 | Rust foundation | Lean library with no server and no imposed I/O, command line | ⬜ |
| 4 | E4 | Source format | Identified elements, canonical form, diagnostics with line numbers and as JSON | ⬜ |
| 5 | E5 | Merise CDM | Seed taken over, own rendering, shipped with a first simple engine | ⬜ |
| 6 | E3 | Layout engine | Own layered engine, iterated against the benchmark until it beats the references | ⬜ |
| 7 | E6 | AI loop and pinnable layout | An agent produces real diagrams; position, layer, order and attach side can be pinned | ⬜ |
| 8 | E10 | Tooled roadmap | Validates existing ROADMAP.md files, enforces states, agent instructions, after-the-fact check | ⬜ |
| 9 | E12 | Portfolio | Declared projects, work queue, text and JSON output, allow-listed publishable export | ⬜ |
| 10 | E9 | Second notation | A UML view on the same elements, cross-checks | ⬜ |
| 11 | E8 | Desktop and mobile application | Diagram preview, portfolio view, conversation thread per point | ⬜ |
| 12 | E13 | Agent control | Agent sessions opened from the application, through adapters | ⬜ |
| 13 | E11 | Graphical editing | Every human gesture rewrites the text in canonical form | ⬜ |

## Stage 1 — Corpus and exchange format  [E1]
1.1 ✅ [P1] Benchmark exchange format: abstract graph as JSON (nodes and their sizes, edges, optional attach sides and constraints), documented schema, validator that refuses with a precise message. Sets up the Rust workspace. — Done 2026-10-06. Two files, `foregram-graph` and `foregram-layout`, the layout repeating nothing of the graph so an engine can only omit or invent, never alter. Integers only, in hundredths of a pixel (rejected: whole pixels, too coarse for text metrics and centring; 1/64, less readable), y pointing down. Flat graphs, rectangular nodes, polyline routes, loops and parallel edges allowed, edge labels with an anchor (Merise cardinalities). Constraints: `direction`, `layer`, `order`, attach sides; no pinned position in version 1. The validator checks imposed sides but not `layer`/`order`, which are benchmark measurements. Hand-written strict JSON reader with line and column on every value (rejected: serde, which gives no position for meaning errors and accepts floats). Stable error codes (J, S, G, L), messages and documentation in English, keys in English. Attested by 49 tests including three guards (documentation shows the example files byte for byte, lists exactly the codes the code produces, every code is exercised), each guard checked by mutation; Clippy and rustfmt clean; command tried on empty, directory, random bytes, and a 200,000-node graph (2 s); identical output over two runs. Not verified: WebAssembly build (target not installed); memory use is 13× the input size.
1.2 ✅ [P10] Localisable messages: a diagnostic carries its code, its position and its arguments; the sentence is composed at display time from a per-language catalogue, English as the only language for now; every message of P1 converted; a test guarantees that every code has a catalogue entry. Same design as GIMO's, so the two tools can share catalogues one day. — Done 2026-10-09. No sentence left in the code: a refusal or a command output carries a key and arguments, an argument being either verbatim text or a nested message translated in turn. One catalogue for the whole product, library and benchmark command line, `crates/foregram-graph/messages/en.txt`, one `key = template` per line, `{name}` for an argument, `\n` for a line break. A refusal key is its stable code plus a variant when needed (`J005.zero`); other keys are fragments (`subject.node`). A missing key renders raw instead of failing. Rejected: a Rust type per message (the compiler would check arguments, but the catalogue would no longer be a file a translator can write alone; tests do that check instead); one catalogue per crate. Syntax chosen here, to be confronted with GIMO's agent when GIMO implements its own decision of 2026-10-06. English output unchanged byte for byte, except `0 nodes` instead of `0 node` (English plural rule). Attested by 59 tests, three of them guards (every refusal agrees with its template, also as a debug assertion at creation; every catalogue entry is reached and none is dead; every key written in the code exists), each guard checked by mutation; a French test catalogue shows a language switch. Limits: reading a catalogue stays internal until its own refusals get codes; plural follows the English rule only.
1.3 🔶 [P11] Exchange format v2: ports (side, rank, optional position and size, shared attach point), nested nodes, routes as cubic Bézier curves besides polylines; validator, canonical form and documentation updated.
1.4 🔶 [P2] Public corpus: download and licences: manifest (address, version, SHA-256), replayable script that downloads Rome, North and elk-models into an ignored cache and checks them; licences read and recorded, nothing versioned.
1.5 ⬜ [P12] GraphML import (Rome, North) with `roxmltree`: every node gets the same fixed size, recorded in the manifest; Rome edges oriented in file order; every converted graph validates.
1.6 ⬜ [P13] ELKT reader and import: reader written from the official `ElkGraph.xtext` grammar, with error positions; all 898 elk-models files read; properties kept as typed key-value pairs; ports, nesting and sizes carried into format v2.
1.7 ⬜ [P3] In-house corpus: real CDMs converted with their links, plus two hard cases (central entity with eight associations, reflexive association); one set stays local, outside the repository.

## Stage 2 — Measurement  [E14]
2.1 ⬜ [P4] References: ELK and a recent Graphviz, at pinned versions, returning coordinates in a common JSON; failures are listed, not hidden.
2.2 ⬜ [P5] Metrics: crossings, bends, edge length, overlaps, area and proportions, written as product code, with a textual report of the layout; tests on cases with known values.
2.3 ⬜ [P6] Neutral rendering: SVG and PNG drawn from the coordinates alone, same look for every engine, to compare the layout and not the style.
2.4 ⬜ [P7] Board and report: graphs side by side and an aggregated table of metrics, regenerated by a single command.
2.5 ⬜ [P8] Determinism: same input, same bytes, over two runs then over two environments where possible; comparison recorded.
2.6 ⬜ [P9] Bar to beat: first complete pass, ELK and Graphviz figures recorded with the exact version of each engine, as the target of the layout engine.
