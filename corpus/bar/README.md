# The bar to beat

Recorded on 2026-10-14 from one full run of `foregram-bench board
corpus/cache corpus/board`, with Graphviz 16.1.0 (`dot`) and ELK 0.12.0 as
pinned in `corpus/manifest.txt`, on x86-64 Linux, glibc 2.39, OpenJDK
21.0.12.1, rustc 1.97.1. The same run in a Debian 13 container (glibc 2.41,
rustc 1.99.0) gave the same bytes. These are the figures ForegramPM's own
layout engine has to beat; the measures are defined in the section
"Metrics" of `docs/exchange-format.md`, the board in "Board and report".

| File | Content |
|---|---|
| `report.md` | The report as `board` wrote it: per corpus, graphs laid out and refused by each engine, four statistics per measure, and the paired comparison on the graphs both engines laid out |
| `digest.txt` | One SHA-256 per group of files of that run, from `corpus/fingerprint.sh digest`; a new run that gives the same lines reproduces the bar |
| `refused.txt` | Every graph an engine refused, with the codes and their counts, one line per engine and graph, taken from the `refused` entries of `metrics.json` |

## What is out, and why

An engine's figures cover the graphs it laid out; the paired comparison
covers the graphs both laid out. Rome, North and the in-house models are
complete under both engines. On elk-models:

- **ELK refuses 20 of 898**: 19 Ptolemy models whose ports it places off the
  border of a leaf too small to hold them (`L015` — the format does not let
  an engine grow a node, and the import does not carry
  `nodeSize.constraints`), and `tests/layered/connected_components/compound06`,
  on which ELK itself throws (`R003`).
- **dot refuses 339 of 898**: 310 where several edges share a port (`L016`,
  dot has no ports), 26 compound graphs on which it fails with
  "triangulation failed" (`R003`), 3 where it grows a 1 × 1 leaf (`L009`).

The own engine must lay out every graph. On a graph a reference refused it
has nothing to beat, and the comparison says so rather than counting it.

**ELK ties.** On 46 elk-models graphs ELK routes a self-loop between two
opposing sides of a node over or under it depending on the JVM's identity
hashes; the bench pins that choice with `-XX:hashCode=2` (section
"Determinism"). The recorded side is the one that flag gives; it is neither
better nor worse than the other.

## The in-house models

The four `corpus/mcd/` models carry the provisional node sizes of their
import (longest line × fixed advance). They will be re-recorded when the
compiler measures text with the embedded font, in the stage "Merise CDM".

## The rule the engine starts from

On each corpus and against each reference, the engine beats the reference
when, on the graphs both laid out:

1. it wins the paired comparison on **crossings** — more graphs with fewer
   crossings than graphs with more;
2. its total **edge length** and total **area** do not exceed 110 % of the
   reference's;
3. its total **bends** do not exceed ELK's; dot is not compared on bends,
   its splines count none.

The engine is expected to beat both references on every corpus. The rule is
a starting point for the stage "Layout engine", which may revise it with a
dated note here.

## Checking the bar

```
corpus/fetch.sh
cargo run --release -p foregram-bench -- board corpus/cache corpus/board
corpus/fingerprint.sh sums corpus/board > sums.txt
corpus/fingerprint.sh digest sums.txt | diff - corpus/bar/digest.txt
```

No difference means the run reproduces the bar byte for byte. The bar is
recorded again when the manifest pins another engine version, when a corpus
changes, or when a measure changes; `report.md` then says which.
