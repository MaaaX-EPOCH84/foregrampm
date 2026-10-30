# Exchange format of the layout benchmark

Two JSON files describe one benchmark case:

- the **graph** (`foregram-graph`): what a layout engine receives —
  rectangular nodes, which may contain other nodes, their ports, edges,
  their labels, and whatever the author chose to pin down;
- the **layout** (`foregram-layout`): what an engine returns for that
  graph — the position of every node and port, the size of every
  container, the route of every edge, the position of every label.

The layout repeats nothing from the graph. An engine therefore cannot
alter a size or an edge; it can only forget an element or cite one that
does not exist, and the validator reports it.

By convention a graph is named `<name>.graph.json` and its layouts
`<name>.<engine>.layout.json`.

This format serves the benchmark and the layout engine. It is not the
format diagrams are written in: that one is `docs/source-format.md`, and
the compiler turns it into a graph.

The product crates, `foregram-graph` and `foregram-render`, take bytes or
strings and return values: they open no file, read no clock or
environment, and write to no stream. The executables do the input and
output. A test reads both crates' sources and refuses any use of
`std::fs`, `std::env`, `std::time`, `std::io`, `std::process`, `std::net`,
`std::thread` or of a printing macro outside unit tests.

Two executables read and write these files. **`foregram`**, the product
command line, reads the exchange format only: it validates, rewrites in
canonical form, measures and draws. **`foregram-bench`** converts the
third-party formats (GraphML, ELK text, conceptual data models) into the
exchange format and drives the reference engines. A converted graph goes
from one to the other through the standard input:

```
foregram-bench graphml g.graphml | foregram validate -
```

This is version 2 of the format. Version 1 files are still read: everything
version 2 adds is optional. The library writes version 2.

## Units and axes

- Every length is an **integer**, in **hundredths of a pixel**: a node
  120 pixels wide has `"width": 12000`.
- The x axis points right, the y axis **points down**.
- The position of a rectangle, node or label, is that of its top-left
  corner.
- **A node is positioned in its parent**: a top-level node relative to
  the drawing, a child relative to the top-left corner of its container.
  **A port is positioned in its node.** **Routes and edge labels are
  absolute.**
- A dimension ranges from 1 to 1,000,000,000; a coordinate from
  −1,000,000,000 to 1,000,000,000.

There is no floating-point number anywhere: that is what lets two
machines produce the same bytes.

## Strict JSON

The text is JSON (RFC 8259), with four extra refusals:

- a number is an integer, with no fraction or exponent, no leading zero,
  and `-0` is written `0`;
- a key appears only once in an object;
- a key the format does not know is refused;
- the text is UTF-8, without a byte order mark.

Keys may come in any order.

## The graph

| Key | Type | | Meaning |
|---|---|---|---|
| `format` | string | required | `"foregram-graph"` |
| `version` | integer | required | `1` or `2` |
| `name` | identifier | required | Name of the graph; the layout cites it |
| `direction` | word | optional | Direction in which layers follow each other: `right`, `down`, `left` or `up` |
| `nodes` | array | required | The top-level nodes |
| `edges` | array | required | All the edges, whatever the depth of their nodes |

A **node**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | Unique among all nodes, at every depth |
| `width`, `height` | integers | leaf only | Dimensions of the rectangle; required for a node without children, refused for a container |
| `layer` | integer | optional | Imposed layer, from 0 to 1,000,000 |
| `order` | integer | optional | Imposed rank within the layer; requires `layer` |
| `children` | array | optional | The nodes this node contains; a node with children is a **container** |
| `padding` | integer | container only | Space kept between the border of a container and its children |
| `ports` | array | optional | The ports of the node |
| `ports_ordered` | boolean | optional | `true`: on each side, the ports follow each other in the order of the list |

A **port**, a named attach point on the border of its node:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | Unique among all ports of the graph |
| `side` | word | optional | Imposed side: `north`, `south`, `east` or `west` |
| `x`, `y` | integers | optional | Imposed anchor point, relative to the top-left corner of the node; requires `side`, and lies on the border of that side |
| `width`, `height` | integers | optional | Box of the port, centred on its anchor point; without them the port is a point |

An **edge**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | Unique among edges |
| `source`, `target` | identifiers | required | Start node, end node |
| `source_port`, `target_port` | identifiers | optional | A port of the start node, of the end node |
| `source_side`, `target_side` | words | optional | Imposed side of the node, for an end without a port: `north`, `south`, `east` or `west` |
| `labels` | array | optional | The labels of the edge |

A **label**:

| Key | Type | | Meaning |
|---|---|---|---|
| `width`, `height` | integers | required | Dimensions of the rectangle |
| `at` | word | required | End the label relates to: `source`, `center` or `target` |

Rules:

- An **identifier** is a non-empty string, without control characters,
  of at most 255 characters.
- Node identifiers, port identifiers and edge identifiers are three
  separate spaces: a node, a port and an edge may carry the same one.
- `source` and `target` refer to nodes that exist; `source_port` refers
  to a port of `source`, `target_port` to a port of `target`.
- An end has a port or a side, never both: with a port, the side is the
  port's.
- `ports_ordered` requires a `side` on every port of the node, since the
  order is read side by side.
- A container cannot position its ports: its size is not known before
  the layout.
- A **loop** (an edge from a node to itself) is allowed, as are several
  edges between the same two nodes, and an edge between a node and one of
  its own ancestors or descendants.
- A graph without nodes is valid.

### What the author pins down

Everything below is optional. Without a constraint, the engine decides.

- `direction` orients the layout.
- `layer` puts a node in a layer. Layers are counted from 0 along
  `direction`: two nodes with the same `layer` sit side by side, a
  smaller `layer` comes first.
- `order` ranks a node within its layer, top to bottom for `right` and
  `left`, left to right for `down` and `up`. Two nodes cannot share the
  same `layer` and the same `order`.
- `source_side` and `target_side` impose the side through which an edge
  leaves or reaches a node. A corner belongs to both of its sides.
- A port's `side` imposes the side of the node it sits on; `ports_ordered`
  imposes the order of the ports on each side; a port's `x` and `y` impose
  its exact anchor point. Those are the four port constraint levels of
  ELK: free, fixed side, fixed order, fixed position — but each port may
  be pinned on its own.

The validator checks imposed sides and port positions, because they can
be read off the layout. It does not check `layer`, `order` or the order
of ports: whether an engine honoured them is a benchmark measurement, not
a validity condition.

## The layout

| Key | Type | | Meaning |
|---|---|---|---|
| `format` | string | required | `"foregram-layout"` |
| `version` | integer | required | `1` or `2` |
| `graph` | identifier | required | The `name` of the graph laid out |
| `engine` | object | required | The engine that produced the layout |
| `nodes` | array | required | Every node, at every depth, in one flat list |
| `edges` | array | required | The route of every edge |

The **engine**: `name` and `version`, required, and `options`, optional,
which says with which settings it ran. Enough to replay the layout.

A **placed node**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | The node of the graph |
| `x`, `y` | integers | required | Top-left corner, relative to the parent |
| `width`, `height` | integers | container only | The size the engine found for a container; refused for a leaf, whose size is in the graph |
| `ports` | array | optional | The anchor point of every port: `id`, `x`, `y`, relative to the node |

A **routed edge**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | The edge of the graph |
| `route` | array of segments | required | The start point, then segments to the end; at least two elements |
| `labels` | array of points | optional | Position of every label, in the order of the graph |

A **point** is written `[x, y]`. A **segment** is either a point, for a
straight line to that point, or three points `[[x, y], [x, y], [x, y]]`,
for a cubic Bézier curve: two control points, then the end point. The
first element of a route is always a point. A version 1 route, all
points, is a valid version 2 route.

Rules, checked by confronting the layout with its graph:

- every node of the graph is placed once, and no other; a container has a
  size, a leaf has none;
- every child lies inside its parent, `padding` kept;
- every port of a node is placed once, and no other; its anchor lies on
  the border of its node, on its side if it has one, and at its position
  if the graph fixes one;
- every edge of the graph is routed once, and no other;
- a route starts on the border of the start node and ends on the border
  of the end node, on the imposed side if there is one — or, for an end
  with a port, on the port: its anchor point, or anywhere in its box;
- an edge has as many placed labels as the graph gives it.

Overlapping nodes or crossing edges do not make a layout invalid: that is
what the benchmark measures.

## Canonical form

The library always writes a file the same way: keys in the order of the
tables above, one node and one edge per line, the children of a container
on the following lines with one more level of indentation, two-space
indentation, optional keys omitted when they have no value, a final
newline. Inside a
string, only the quote, the backslash and control characters are escaped.

Two equal contents therefore give the same bytes, and reading a canonical
form then writing it back changes nothing.

```
foregram canon <file.json|file.fg>
```

reads a graph or a layout, told apart by its `format` key, and writes it in
canonical form, version 2, on the standard output. A file whose `format`
does not name a layout is read as a graph, and its refusals say what is
wrong. A refused file is not written. A file that does not open with `{`
or `[` is a source file of a diagram, rewritten by the rules of
[`source-format.md`](source-format.md).

## Validating

```
foregram [--lang <code>] [--json] validate <graph.json> [<layout.json>...]
```

The graph is validated, then each layout is validated and confronted with
the graph. Every file is read, even after a refusal. A file named `-` is
the standard input, which a call reads once at most; its refusals name it
`<stdin>`.

| Exit code | Meaning |
|---|---|
| 0 | Everything is valid |
| 1 | At least one file is refused |
| 2 | Wrong call, or unreadable file |
| 3 | Internal error: an invariant of the product broke, the file may be right |

An internal error is a fault of the product, not of the file: it is
reported like a refusal, with a code of the family `INT` (see
[Internal errors](#internal-errors)), and its exit code wins over the
others when one run mixes them, since a fault makes the other verdicts
suspect. A program that reads 1 corrects its file, 2 its call, and on 3
stops and reports, touching nothing.

`--lang` chooses the language of the messages: `en`, the default, or
`fr`; any other is a wrong call. Without it the messages are in English,
whatever the language of the system: no environment variable is read. `--json` writes results and refusals as
JSON, see [Diagnostics as JSON](#diagnostics-as-json). Both come before the
command, in either order, and so do they for `canon`, `metrics` and
`render`, which share these exit codes.

A refusal is written on the error output:

```
example.graph.json:10:5: error[GPH003]: edge "e1": `target` refers to node "B", which does not exist
```

that is the file, the line, the column, the code, the message. The
column counts characters, not bytes. The **code** is stable from one
version to the next and does not depend on the language of the message:
it is what a program must read.

The message is composed at display time from a per-language catalogue,
[`messages/en.txt`](../crates/foregram-graph/messages/en.txt): a refusal
carries its code and its arguments, not its sentence. The French catalogue,
[`messages/fr.txt`](../crates/foregram-graph/messages/fr.txt), has the same
keys and the same arguments for everything `foregram` says; the benchmark
speaks English only. The same refusal in French:

```
example.graph.json:10:5: erreur[GPH003] : lien « e1 » : `target` désigne le nœud « B », qui n'existe pas
```

with the same file, line, column and code; the spaces before `:` and
inside « » are no-break spaces (U+00A0), as French typography wants. A count has two entries,
`.one` and `.other` (`count.node.one`, `count.node.other`), and the
language picks one at display time: English takes `.one` for 1, French
for 0 and 1. A library user chooses the language by
`Catalog::for_language("fr")`, or `Catalog::english()` and
`Catalog::french()`; `Catalog::LANGUAGES` lists the codes.

A JSON syntax error stops the reading at the first one. Structure errors
are all reported at once, and so are meaning errors, but the latter are
only looked for in a file whose structure is right.

### Codes

A code is three capital letters naming its family, then three digits.
The number `000` is reserved and never given, in any family. Each
family was once a single letter; the numbers did not change:

| Before | Now | Family |
|---|---|---|
| `J` | `JSN` | JSON syntax |
| `S` | `SCH` | structure (schema) |
| `G` | `GPH` | meaning of a graph |
| `L` | `LYT` | meaning of a layout |
| `X` | `GML` | GraphML import |
| `E` | `ELK` | ELK text reader and import |
| `M` | `MCD` | conceptual data model import |
| `R` | `REF` | reference engines |
| `V` | `DRW` | rendering (drawing) |
| — | `INT` | internal errors, new with the three-letter families |

The source format has its own family, `SRC`, formerly `T`, documented
in [`source-format.md`](source-format.md).

JSON syntax:

| Code | Refusal |
|---|---|
| JSN001 | Text that is not UTF-8, or byte order mark |
| JSN002 | Unexpected character or word |
| JSN003 | Unexpected end of text, unterminated string |
| JSN004 | Non-integer number |
| JSN005 | Malformed number: leading zero, `-0`, lone `-` |
| JSN006 | Integer too large |
| JSN007 | String: invalid escape, control character |
| JSN008 | Duplicate key in an object |
| JSN009 | Nesting too deep |
| JSN010 | Content after the value |

Structure, for both formats:

| Code | Refusal |
|---|---|
| SCH001 | Unexpected type |
| SCH002 | Unknown key |
| SCH003 | Required key missing |
| SCH004 | Unexpected `format` |
| SCH005 | Unsupported `version` |
| SCH006 | Word outside the allowed list |
| SCH007 | Identifier empty, too long or with a control character |
| SCH008 | Integer outside its range |

Graph:

| Code | Refusal |
|---|---|
| GPH001 | Node identifier already used |
| GPH002 | Edge identifier already used |
| GPH003 | Edge to a node that does not exist |
| GPH004 | `order` without `layer` |
| GPH005 | Two nodes with the same `layer` and the same `order` |
| GPH006 | Port identifier already used |
| GPH007 | Edge to a port that does not exist, or that belongs to another node |
| GPH008 | A side and a port given on the same end of an edge |
| GPH009 | `ports_ordered` on a node whose ports lack a `side` |
| GPH010 | Port position without a side, off its side, or on a container |
| GPH011 | Size missing on a leaf, or given on a container |
| GPH012 | `padding` on a node without children |

Layout:

| Code | Refusal |
|---|---|
| LYT001 | The layout cites another graph |
| LYT002 | Node not in the graph |
| LYT003 | Node placed twice |
| LYT004 | Node not placed |
| LYT005 | Edge not in the graph |
| LYT006 | Edge routed twice |
| LYT007 | Edge without a route |
| LYT008 | Route with fewer than two points |
| LYT009 | Route end off the border of its node |
| LYT010 | Route end off the imposed side |
| LYT011 | Number of placed labels differs from the graph |
| LYT012 | Child outside its parent |
| LYT013 | Size missing on a placed container, or given on a placed leaf |
| LYT014 | Port not in the graph, not placed, or placed twice |
| LYT015 | Port anchor off the border, off its side, or off its fixed position |
| LYT016 | Route end off its port |
| LYT017 | Route starting with a curve |

## Diagnostics as JSON

```
foregram --json <command> ...
```

With `--json`, every command writes **one JSON object per line**, all on
the standard output, in the order of the files: refusals and results
alike. The error output carries only the usage, as text, on a wrong call
(exit code 2). Exit codes are the same as without `--json`. Every object
reads with the strict reader of this format: integers only, each key
once.

A refusal, for example of the graph `refuse-lien-orphelin.graph.json`
read from the standard input:

```json
{"kind": "diagnostic", "file": "<stdin>", "code": "GPH001", "line": 7, "col": 5, "key": "GPH001.again", "args": {"line": "6", "id": "A"}, "text": "node identifier \"A\" already used (line 6)"}
{"kind": "diagnostic", "file": "<stdin>", "code": "SCH008", "line": 7, "col": 5, "key": "SCH008", "args": {"field": {"key": "field.at", "args": {"subject": {"key": "subject.node", "args": {"id": "A"}}, "key": "height"}}, "value": "0", "min": "1", "max": "1000000000"}, "text": "node \"A\": `height`: 0 outside the range 1 to 1000000000"}
{"kind": "diagnostic", "file": "<stdin>", "code": "GPH003", "line": 10, "col": 5, "key": "GPH003", "args": {"subject": {"key": "subject.edge", "args": {"id": "e1"}}, "key": "target", "node": "B"}, "text": "edge \"e1\": `target` refers to node \"B\", which does not exist"}
```

| Key | Value |
|---|---|
| `kind` | `"diagnostic"` |
| `file` | The file, as the text output names it (`<stdin>` for `-`) |
| `code` | The stable code: **what a program reads** |
| `line`, `col` | The position, from 1, the column in characters; `null` when the refusal has no place in a text (a rendering refusal, an internal error) |
| `key` | The catalogue key of the message: the code, plus a variant where the code has several sentences (`GPH001.again`) |
| `args` | The arguments of the message, in the order given |
| `text` | The sentence, composed in the language chosen by `--lang` |

An argument is a string, or a message of its own, written
`{"key": …, "args": {…}}` and without a sentence: the sentence is
composed once, under `text`. Numbers in arguments are strings, as the
catalogue receives them. A count is written with the key it has in every
language, without `.one` or `.other` (`count.node`), and its number in
`n`. Under `--lang`, only `text` changes: the rest of every object is the
same in every language.
The keys and arguments of a message may change from one version to the
next; the code may not.

The other objects have the same `kind`, `file`, `key`, `args` and
`text`, and no code or position:

| `kind` | Written by | Adds |
|---|---|---|
| `valid` | `validate`, for a valid graph or a valid layout; `check`, for a well-formed [source file](source-format.md#checking) | — |
| `metrics` | `metrics`, for each layout | `metrics`: one integer per measure, as in [`metrics.json`](#board-and-report) |
| `rendered` | `render`, once the image is written | — |
| `error` | Any command, for a file that cannot be read or written, or a layout not checked because its graph is refused | — |

`canon` is the one exception to one object per line: its result spans
several lines, with the canonical document as it is under `document`,
shifted by one level:

```
{
  "kind": "canonical",
  "file": "g.json",
  "document": {
    "format": "foregram-graph",
    ...
  }
}
```

A source file rewritten by `canon` keeps to one object on one line, its
canonical text as a string under `document`. A refused file gives its
`diagnostic` objects instead, one per line.

## Importing GraphML

The Rome and North corpora are GraphML files; the benchmark converts them
in memory, on every run, and keeps no converted copy.

```
foregram-bench graphml <file.graphml>
foregram-bench check-corpus <cache>
```

`graphml` converts one file and writes the graph, in canonical form, on
the standard output. `check-corpus` converts every graph of Rome and
North in the cache filled by `corpus/fetch.sh` (`corpus/cache`), and
compares the number of nodes and edges of each with what the archive's
`Graph.log` announces: a source independent of the reader, which sees what
an import would have lost and the validator cannot. Exit codes as above;
a graph that differs from its `Graph.log` counts as refused.

The conversion:

- the graph is named after the file, without `.graphml`; nodes and edges
  keep their GraphML identifiers;
- every node is 4950 × 3500 (49.5 × 35 pixels), the exact proportion of an
  A4 sheet, 99/70, in landscape; GraphML gives no size, and the same size
  is recorded in `corpus/manifest.txt`;
- an edge goes from its `source` to its `target` as written, directed or
  not: undirected edges keep the order of the file; no `direction` is set.

Only what the exchange format can say is accepted: a `graphml` root, with
or without the GraphML namespace, holding one `graph`, holding `node` and
`edge` elements without content. Anything else — `key`, `data`, `port`,
`hyperedge`, a nested graph, an unknown attribute — is refused rather than
dropped, since a silently trimmed graph would skew the benchmark.
Attributes in the `xsi` namespace, comments and processing instructions
are ignored; the document type declaration is read but never fetched.

A refusal points into the `.graphml` file. Once the file is read, the
graph is validated like any other, and those refusals point into the
`.graphml` as well.

### Codes

GraphML import:

| Code | Refusal |
|---|---|
| GML001 | Text that is not UTF-8 |
| GML002 | Malformed XML, with the parser's own detail |
| GML003 | Root element other than `graphml` |
| GML004 | Not exactly one `graph` in `graphml` |
| GML005 | Element not supported, or text where none is expected |
| GML006 | Attribute not supported |
| GML007 | `id` of a node or an edge, `source` or `target` missing |
| GML008 | `edgedefault` or `directed` with an invalid value |

## Importing ELK text

The elk-models corpus is written in ELK's text format, `.elkt`. The
benchmark reads it with its own reader, written from the official grammar
`ElkGraph.xtext` of `eclipse-elk/elk` (`plugins/org.eclipse.elk.graph.text`,
commit `60b6846`), and imports the result in memory, like GraphML.

```
foregram-bench elkt <file.elkt>
foregram-bench check-corpus <cache>
```

`elkt` converts one file and writes the graph, in canonical form, on the
standard output; what the graph could not carry is counted on the standard
error. `check-corpus` also converts every `.elkt` under `elk-models` in the
cache. There is no `Graph.log` there: the independent source is
`corpus/manifest.txt`, next to the cache, which gives the number of files
(`count`) and the number that must import (`imported`); a difference counts
as a problem, and every imported graph is validated.

The reader follows the grammar to the letter: keywords (`node`, `port`,
`edge`, `label`, `layout`, `position`, `size`, `start`, `end`…) are not
identifiers unless escaped with `^`, which is removed (`^port.side` is the
key `port.side`); a `layout` group takes each of its keys at most once, in
any order; a property key has no blank between its segments; a string is
read without its quotes and with Java escapes decoded; numbers are kept as
written. The reader keeps everything, properties included, as typed values;
the import then takes what the format can say.

The conversion:

- the graph is named after the file, without `.elkt`;
- **nesting** is kept: a `node` inside a `node` is a child; the root of the
  file is the graph, not a node, so an edge to the root, or a port on the
  root, is refused;
- **identifiers**: ELK's are unique within their container, the format's in
  the whole graph. Nodes keep their written identifier when all of them are
  distinct, otherwise every node takes its path from the top level
  (`parent.child`); ports likewise, prefixed with their node's identifier
  (`node.port`) when needed; edges likewise, prefixed with the path of the
  container they are written in. An edge without an identifier gets the
  first free `e1`, `e2`…, in file order;
- **references** (`edge a.b -> c`) resolve as in ELK: relative to the
  container the edge is written in, then to each of its ancestors, then
  absolute;
- **sizes**: `layout [size: w, h]` of a leaf, in pixels, becomes `width` and
  `height` in hundredths of a pixel, rounded to the nearest, halves away from
  zero. A leaf without a size — ELK gives it size zero — gets **1 × 1**, the
  smallest size the format allows, and is counted. A container's size is not
  carried: the layout gives it;
- **ports**: kept with their box (`layout [size]`). What else is carried
  depends on the node's `portConstraints`, read as ELK does it: under `FREE`
  (or none) nothing, even a written `side`; under `FIXED_SIDE` and above, the
  `side` — the written one, or the one ELK infers from the port's position
  (`ElkUtil.calcPortSide`, the layout direction, `RIGHT` by default, breaking
  ties); under `FIXED_ORDER`, `ports_ordered` and the list in ELK's order,
  side by side, by `port.index` when every port has one, else by position
  clockwise, else as written; under `FIXED_POS`, the anchor point: the centre
  of the port box, which ELK places by its corner, brought onto the border of
  its side, a missing position meaning (0, 0) as in ELK. A container cannot
  pin its ports in the format: the side is kept, the position dropped and
  counted. `FIXED_RATIO` is refused;
- `direction` of the root (`UNDEFINED` meaning none);
- **edge labels** with a size become labels, `edgeLabels.placement` giving
  `at` (`HEAD` the target, `TAIL` the source, otherwise the centre); labels
  without a size, and all labels of nodes and ports, are not carried and are
  counted;
- everything else — layout options, node and port positions, edge sections,
  which are results of a layout — is not carried.

Option keys match ELK's whole identifier or any suffix of it from a dot:
`org.eclipse.elk.port.side`, `elk.port.side`, `port.side`, `side`. When a key
is given twice on one element, the last one wins, as in ELK.

A refusal points into the `.elkt` file. A syntax error stops the reading at
the first one; import refusals are all reported; the graph is then validated
like any other, and those refusals point into the `.elkt` as well.

### Codes

ELK text reader:

| Code | Refusal |
|---|---|
| ELK001 | Text that is not UTF-8 |
| ELK002 | Unterminated string or comment |
| ELK003 | Unexpected character |
| ELK004 | Unexpected token or end of text, with what was expected |
| ELK005 | Unknown escape sequence in a string |
| ELK006 | A `layout` key given twice |

ELK import:

| Code | Refusal |
|---|---|
| ELK007 | Edge end that refers to no node or port |
| ELK008 | Hyperedge: more than one source or one target |
| ELK009 | Number that cannot be carried in hundredths of a pixel |
| ELK010 | Option value outside the words the import knows |
| ELK011 | Edge end on the root graph itself, or ports on the root |

## Importing a conceptual data model

The in-house corpus is made of Merise conceptual data models (CDM),
written in the small text format of the first prototype, `.mcd`; they are
in `corpus/mcd/`. Models that cannot be published stay in `corpus/local/`,
which Git ignores.

```
foregram-bench mcd <file.mcd>
```

`mcd` converts one file and writes the graph, in canonical form, on the
standard output.

The format, keywords in French as in the models it describes:

```
# a comment, in column one
entite MEMBRE
  #id_membre        a leading # marks the identifier
  nom               a property
association PRET
  date_pret         a property carried by the association
  MEMBRE 0,n        a link: entity, then cardinality
```

A block starts in column one, its content is indented with spaces or tabs.
A cardinality is `0,1`, `0,n`, `1,1` or `1,n`.

The conversion:

- the graph is named after the file, without `.mcd`; its `direction` is
  `right`;
- every entity and every association is a node, named as written, in file
  order;
- every link is an edge from the entity to the association, named
  `ASSOCIATION.ENTITY`; a reflexive association links the same entity
  twice, and the second edge is named `ASSOCIATION.ENTITY.2`;
- the cardinality is a label against the entity (`at: source`).

Sizes follow a provisional rule, until text is measured with the embedded
font: a node is as wide as its longest line (its name or a property,
without `#`) at 7 pixels per character, as high as its lines at 18 pixels
each, plus 8 pixels of margin on every side; a cardinality is 7 pixels per
character plus 2 on each side, 18 pixels high.

### Codes

| Code | Refusal |
|---|---|
| MCD001 | Text that is not UTF-8 |
| MCD002 | Line in column one other than `entite NAME` or `association NAME` |
| MCD003 | Indented line before any block |
| MCD004 | Link inside an entity |
| MCD005 | Indented line that is neither a property nor a link |
| MCD006 | Entity or association declared twice |
| MCD007 | Link to something that is not a declared entity |
| MCD008 | Association with fewer than two links |

## Reference engines

The benchmark measures its own layout engine against two references, run at
pinned versions and brought back into the layout format: `dot`, of Graphviz,
and ELK Layered. `corpus/fetch.sh` installs both in the cache from
`corpus/manifest.txt` — the Graphviz Debian package of the pinned release,
unpacked without installing; the ELK jars from Maven Central, each with its
SHA-256, and the benchmark's small Java driver compiled against them with
`javac`. Running ELK needs a Java runtime on the `PATH`; nothing else is
installed on the machine. The driver is launched with
`-XX:+UnlockExperimentalVMOptions -XX:hashCode=2`, a HotSpot flag that gives
every object the same identity hash: without it ELK routes some self-loops
differently from one launch to the next (section "Determinism"). The
manifest pins ELK, not the JVM; a runtime that rejects the flag refuses to
start, which the bench reports as REF003.

```
foregram-bench layout <engine> <cache> <graph>
foregram-bench layout-corpus <engine> <cache> <out>
```

`<engine>` is `dot` or `elk`. `layout` takes one graph — a `.graph.json`, or
a `.graphml`, `.elkt` or `.mcd` file converted on the fly —, lays it out and
writes the layout on the standard output, in canonical form, refused or not;
refusals go on the error output. `layout-corpus` lays out every graph of Rome,
North, elk-models and the in-house corpus, writes each layout under
`<out>/<engine>/<corpus>/`, and lists every failure, with its code, in
`<out>/<engine>/failures.txt`; a summary per corpus says how many graphs were
laid out and valid, how many failed. **A failure is listed, never hidden**: a
graph the engine rejects, one whose output cannot be read, one whose layout
the validator refuses, each appears with its reason, and the layout file is
written whenever it could be built. Exit codes as above; one failure makes
the run exit with 1.

Before anything runs, the engine's version is read — `dot -V`, the name of
the ELK core jar — and compared with the manifest: a missing engine or another
version stops the command with exit code 2.

The layout's `engine` records the name, the pinned version and the settings,
enough to replay it. Both engines run with their defaults, plus what the
graph says:

- **`dot`**: the graph becomes a `digraph` with `compound=true`, `rankdir` from
  `direction`, nodes `shape=box fixedsize=true` with their size in inches, a
  container as a `cluster` whose `margin` is its `padding` (8 points, dot's
  default, when it has none), edges without arrowheads. An imposed side, of
  an edge or of a port, becomes a compass point (`n`, `e`, `s`, `w`); an edge
  to a container attaches to its first leaf with `lhead` or `ltail`, which
  dot clips at the cluster border; a label becomes an empty HTML table of
  fixed size, in whole points. `dot -Tjson0` gives the result: node centres
  and cluster boxes in points, with the y axis pointing up, which the import
  turns over; one point is one pixel. dot has no ports: the layout gives each
  port the position the graph fixes, else the end of the first edge that uses
  it, else the middle of its side — so a port used by two edges that dot
  attached at different points is refused by the validator, and listed.
  Layers, orders and the order of ports are not passed.
- **ELK**: the graph goes to the driver as it is — nesting, `padding`,
  `direction`, sizes, ports and labels. Each node's `portConstraints` is the
  most demanding level its ports ask for: `FIXED_POS` with a pinned position,
  `FIXED_ORDER` with `ports_ordered` (the ports then carry their `index`),
  `FIXED_SIDE` with a side on a port or on an edge end, else `FREE`. A side
  imposed on an edge without a port becomes a nameless port fixed on that
  side, dropped from the layout. ELK places a port by the corner of its box
  and keeps it outside the border; the import sets `portBorderOffset` to
  minus half the box, so the box stays centred on the border as the format
  wants, and an edge ends on the outer edge of the box, inside it. A nested
  graph runs with `hierarchyHandling=INCLUDE_CHILDREN`, without which ELK does
  not route an edge across a container. Layers and orders are not passed.
  The driver returns absolute coordinates; a child's position relative to
  its parent is the difference of the two rounded absolutes, so that borders
  and route ends fall on the same hundredth.

Both engines compute in floating point, `dot` clips its curves by
approximation and attaches a compass point exactly one point outside the
node: a route end lands up to a point from the border it should touch, and
the validator would refuse every layout. **A route end at most a pixel and a
half away from its target is brought onto it**: the border of its node, the
imposed side, the anchor of a point port, the box of a sized port. Every
move is counted and the largest one is reported, by graph with `layout`, by
corpus with `layout-corpus`. An end farther away stays where the engine put
it, and the validator lists it.

What the engines cannot do with this format is listed the same way. `dot`
fails on some compound graphs (`triangulation failed`), ignores `lhead` on an
edge whose tail is inside the cluster, and gives a sizeless 1 × 1 node a
larger minimum size, so the route ends off it. ELK spreads the ports of a
side evenly and beyond the node when they do not fit, since the format fixes
the size of a leaf, and cannot centre on the border a sideless port whose box
is not square; it also rejects a few compound graphs of its own test corpus.

### Codes

| Code | Refusal |
|---|---|
| REF001 | Engine not found in the cache |
| REF002 | Engine version differs from the manifest |
| REF003 | Engine failed on the graph, with its message |
| REF004 | Engine timed out (60 s) |
| REF005 | Engine output not understood |
| REF006 | Number in the output that cannot be carried in hundredths of a pixel |
| REF007 | ELK changed the size of a leaf |

## Metrics

The benchmark compares engines on numbers read off a layout confronted with
its graph. The layout must pass the validation above; the metrics then
measure what validation does not judge.

```
foregram metrics <graph.json> <layout.json>...
```

The command checks each layout against the graph and writes one line per
measure. A `.graphml`, `.elkt` or `.mcd` graph is converted first by the
benchmark, then read from the standard input. A layout that fails the check is refused, not measured. All
numbers are integers: lengths in hundredths of a pixel, areas in square
hundredths, ratios in thousandths. The same layout gives the same numbers on
every machine.

**Every curve is first flattened** into 16 straight segments, its points
taken at sixteenths of the parameter and rounded to the nearest hundredth;
every measure below reads the polyline obtained. Repeated points are
dropped.

| Measure | Definition |
|---|---|
| crossings | Pairs of segments of two distinct edges that meet in a single point interior to both routes. A route that passes through a vertex of the other — its neighbours on either side — counts once; a route that touches the other and comes back on the same side does not. A point shared by the ends of two edges never counts, nor does an edge with itself |
| shared stretches | Pairs of segments of two distinct edges that are collinear and overlap over a positive length, with the length of all overlaps added up |
| bends | Vertices between two straight segments where the direction changes. A curve, and its junction with a straight segment, is never a bend; a vertex where the route continues straight on is not one either |
| edge length | The length of every route added up, each segment rounded to the nearest hundredth, and the length of the longest route |
| node overlaps | Pairs of nodes, at any depth, whose rectangles overlap over a positive area, with the areas added up. A node and one of its ancestors are not a pair; touching is not overlapping |
| edges through a foreign node | Pairs of an edge and a node that is neither one of its ends nor an ancestor of one of them, where a segment of the route enters the inside of the node. Running along the border does not enter |
| label overlaps | Pairs of a label and a node that is not a container of its edge, of two labels, or of a label and an edge other than its own, where they overlap or the route enters the label |
| extent | The bounding box of every node, route point and label: width, height and area; the proportion, width over height in thousandths (1414 is a landscape 1:√2 sheet); the density, area of the leaves over area of the box, in thousandths |
| constraints broken | Pairs of nodes with a `layer` whose layers and whose centres along `direction` are not in the same order (`right` when the graph has no direction); pairs of nodes of the same layer with an `order` whose orders and whose centres across the direction are not in the same order; pairs of ports of the same side, on a node with `ports_ordered`, whose anchors do not follow the list order clockwise — left to right on the north side, top to bottom on the east, right to left on the south, bottom to top on the west |

## Neutral rendering

A layout is drawn from its coordinates alone, with the same look for every
engine, so that the eye compares the layout and not the style. The drawing
lives in the `foregram-render` crate; the command line exposes it:

```
foregram render <graph.json> <layout.json> <out.svg|out.png> [--scale <n>]
```

The command checks the layout against the graph and writes the drawing
into the output file, SVG or PNG by its extension. A layout that fails the
check is refused, not drawn.

**What is drawn**, in this order, each from the layout's coordinates:

| Element | Drawn as |
|---|---|
| sheet | A white background over the whole view |
| leaf | A light grey rectangle with a dark border, 1 px |
| container | A thin, lighter border, 0.5 px, no fill |
| edge | A dark 1 px line along the route; curves are drawn as curves in the SVG and as the 16-step polyline of the metrics in the PNG, so the image shows what was measured |
| arrow | A dark triangle at the end of the route, 8 px long and 5 px wide, pointing along the last segment of the polyline |
| label | A dashed 0.5 px rectangle, empty: the format carries no text |
| port | A hollow square of 4 px centred on the anchor, or the port's own box when the graph gives one: filled with the sheet's white and outlined with a thin dark border, 0.5 px, so it stands out from the node border and the edge end |

No text is drawn, since the graph carries none. In the SVG, every node, edge
and port carries its identifier in a `<title>`, shown on hover and found by
a search; the drawing's own title is the graph name and the engine.

**The view** is the bounding box of the metrics' extent, plus a margin of
10 px on every side; its origin may be negative. An empty layout gives a
20 × 20 px sheet. Fixed proportions and fixed sizes will come with the
source format.

**SVG.** Coordinates are written in pixels, from the hundredths of the
format by integer arithmetic (`4950` is written `49.5`), never through a
floating-point number. Each element carries a class — `sheet`, `leaf`,
`container`, `edge`, `arrow`, `label`, `port` — styled by one `<style>`
block at the top, so a page that embeds several drawings can restyle them
together. Nothing in the file depends on the machine, the date or the
engine, beyond the title.

**PNG.** The image is rasterised here, in integers: every element is a
polygon — a rectangle, the ring of a border, a quadrilateral per segment of a
line with an octagon at each joint, the dashes of a label, the triangle of an
arrow — filled by the non-zero winding rule, with anti-aliasing from 16
sub-scanlines per row and exact horizontal coverage in 256ths of a pixel.
One grey channel, 8 bits. `--scale` gives the pixels per pixel of the
drawing, 1 by default and 8 at most; an image of more than 50,000,000
pixels is refused. Only the encoding — chunks, CRC, deflate — comes from the
`png` crate, at a pinned version, so the same layout gives the same bytes on
every machine.

### Codes

| Code | Refusal |
|---|---|
| DRW001 | Scale outside 1 to 8 |
| DRW002 | Image larger than 50,000,000 pixels |
| DRW003 | PNG encoder failure |

## Internal errors

The product never stops dead: a case it believes impossible is handled
by a fallback, which the sources mark with a comment `// fallback:` and
its reason, and a guard refuses one without. Where a fallback would hide
a broken invariant — a drawing with a hole, a measure of zero — it is an
internal error instead: a refusal of the family `INT`, without position,
which the command line reports with exit code 3. The file may be right;
what is wrong is the product, and the error asks to be reported.

Today the only such invariant is a layout taken as checked that was not:
`Metrics::measure`, `Scene::build`, `svg` and `png` take a layout that
passed `Layout::check`, and refuse one that leaves something unplaced.
The command line always checks first, so these never come out of it
unless the check itself is at fault.

### Codes

| Code | Fault |
|---|---|
| INT001 | A node has no place in a layout taken as checked |
| INT002 | A port has no place in a layout taken as checked |
| INT003 | An edge has no route in a layout taken as checked |

## Board and report

One command lays out every corpus with both reference engines, measures
every layout and writes what compares them:

```
foregram-bench board <cache> <out> [--layouts <dir>]
```

Without `--layouts`, the command first runs `layout-corpus` for `dot` then
for `elk` into `<out>/layouts/`, which takes minutes. With it, it reads the
layouts that `layout-corpus` already wrote into `<dir>`, failures included.
In both cases it then writes into `<out>`:

| File | Content |
|---|---|
| `metrics.json` | Every graph and engine: its metrics, or its refusals counted by code |
| `report.md` | Per corpus: graphs laid out and refused, refusals by code, four statistics per measure and per engine, and the comparison on the graphs both engines laid out |
| `board.md` | Per corpus, a sample of graphs, each drawn by both engines side by side, with its crossings and bends |
| `svg/<engine>/<corpus>/…svg` | The drawings of the board, with the neutral look; replaced as a whole on every run |

**Outcomes.** A graph is laid out by an engine when its layout file exists
and passes the check against the graph; it is refused otherwise, with the
codes of the check, of the import, or of `failures.txt` when the engine gave
no layout. A layout neither present nor listed among the failures, or
written by another engine version than the one the manifest pins, would make
the board wrong: the command then writes nothing and exits with code 2.
Failures of the engines are data, not errors: the command exits with code 0.

**`metrics.json`** is in the strict JSON of this format. The engines and their
versions come first, then one line per graph and engine, in path order:

```
{
  "format": "foregram-board",
  "version": 1,
  "engines": [
    {"name": "dot", "version": "16.1.0"},
    {"name": "elk", "version": "0.12.0"}
  ],
  "results": [
    {"corpus": "rome", "graph": "rome/r1.graphml", "engine": "dot", "metrics": {"crossings": 1, …}},
    {"corpus": "elk-models", "graph": "elk-models/e1.elkt", "engine": "dot", "refused": {"LYT016": 1}}
  ]
}
```

`metrics` carries one integer per measure of the section "Metrics", under the
names `crossings`, `shared_segments`, `shared_length`, `bends`,
`edge_length`, `longest_edge`, `node_overlaps`, `node_overlap_area`,
`edge_node_overlaps`, `label_overlaps`, `width`, `height`, `area`,
`proportion`, `density`, `layer_violations`, `order_violations`,
`port_order_violations`, in that order. `refused` maps each code to the
number of refusals with it.

**Statistics.** Per measure and per engine, over the graphs that engine laid
out: the total, the median, the 90th percentile and the maximum. Quantiles
take the nearest rank, without interpolation, so they are always values that
were measured: the median is the value of rank ⌈n/2⌉ in increasing order,
the 90th percentile that of rank ⌈9n/10⌉. **The comparison** counts, over the
graphs laid out by both engines only, how often each engine has the lower
value, or both the same, for crossings, bends, edge length and area. An
engine that fails on the hard graphs would otherwise look better on the rest.

**The sample** of the board is fixed, without randomness: per corpus, 8
graphs at regular intervals in path order (the graph of rank ⌊k·n/8⌋ for k
from 0 to 7), all of them when there are 8 or fewer; then, for each engine,
the 3 other graphs, laid out by both, on which that engine is furthest ahead
in crossings, widest gap first, path order between equal gaps.

Nothing written depends on the machine, the date or the time taken: the same
layouts give the same bytes.

**The bar to beat** is a recorded run of `board`, kept under `corpus/bar/`:
`report.md` as the command wrote it, `digest.txt` as `corpus/fingerprint.sh
digest` summarised the whole output, and a `README.md` that says which
graphs each engine refused, how the figures are to be read, and the rule the
layout engine starts from. The layouts and `metrics.json` of that run are not
versioned: `board` regenerates them byte for byte, which `digest.txt`
verifies. The bar is recorded again whenever an engine version in the
manifest changes, or the corpora, or the metrics.

## Determinism

Same input, same bytes. In the Rust code this holds by construction:
integers only, ordered maps, no clock and no environment are read, and the
tests of the rendering and of the board compare two runs byte for byte. The
reference engines are not this code: dot computes in floating point against
the C library of the machine, ELK in a JVM. Their reproducibility is
therefore **measured**, on this machine and in a second environment, and the
result is recorded below so that the figures of the bar to beat are known to
be stable, or known not to be.

`corpus/fingerprint.sh` is the instrument, in shell like `fetch.sh`:

```
corpus/fingerprint.sh env                    # describes the environment
corpus/fingerprint.sh sums <dir> > sums.txt  # SHA-256 of every file under <dir>
corpus/fingerprint.sh digest sums.txt        # one SHA-256 per group of files
corpus/fingerprint.sh compare a.txt b.txt    # differences, group by group
```

`sums` lists every regular file under the directory, path relative and in
byte order, so two outputs of `board` compare as two text files. A **group**
is the first two components of a path — `layouts/dot`, `layouts/elk`,
`svg/dot`, `svg/elk` — or the whole path when it has fewer, as `metrics.json`
: that is the grain at which a difference reads, engine by engine. `digest`
writes one SHA-256 per group, computed over the group's lines of `sums`, with
its number of files: that is what the record below keeps. `compare` exits
with code 1 when anything differs and names the first twenty files of each
group that do.

**The second environment** is a container, `corpus/docker/`: Debian trixie
instead of Ubuntu 24.04, hence another C library and another build of the
JDK, the Rust toolchain of the day, and the same CPU. `corpus/docker/run.sh
<out>` builds the image, copies the sources and the cache in, has `fetch.sh`
check the cache and recompile the ELK driver there, builds the bench, runs
`board` and writes `<out>/sums.txt` and `<out>/env.txt`.

**Recorded comparison, 2026-10-14**, three full runs of `board` with
Graphviz 16.1.0 and ELK 0.12.0, 27 493 files each:

| Group | Run 1 | Run 2 | Run 3 |
|---|---|---|---|
| Environment | Ubuntu 24.04 base, glibc 2.39, OpenJDK 21.0.12.1 (Ubuntu), rustc 1.97.1 | same machine, same day | Debian 13 container on the same machine, glibc 2.41, OpenJDK 21.0.12.1 (Debian), rustc 1.99.0 |
| `layouts/dot`, 13 688 files | `63ecbc18…` | identical | identical |
| `layouts/elk`, 13 713 files | `1e2dca7a…` | `cc653639…`, **46 differ** | identical to run 2 |
| `svg/dot`, 43 files | `2f5aca42…` | identical | identical |
| `svg/elk`, 46 files | `ca4a4489…` | `714b60fc…`, 2 differ | identical to run 2 |
| `metrics.json` | `9d3e2616…` | `81f87cb5…`, differs | identical to run 2 |
| `report.md` | `e1bef8ea…` | `ac753c07…`, differs | identical to run 2 |
| `board.md` | `48ace46b…` | identical | identical |

**dot is reproducible**, run to run and across the two C libraries, on all
13 688 layouts. **The bench itself is**, across the two toolchains: the
container run is identical to run 2 on all 27 493 files, and the SVG,
`metrics.json` and `report.md` only differ where a layout differs.

**ELK is not**, on 46 of the 898 elk-models graphs (45 Ptolemy models and
`examples/edges/insideSelfLoops`), and this is independent of the
environment: the same graph laid out twelve times on the same machine gave
one result ten times and the other twice. Every difference is a self-loop
between ports on two opposing sides of a node, drawn around the top of the
node in one run and around its bottom in the other; the nodes it passes
move accordingly. Crossings and bends are unchanged on all 46; the edge
length changes on 31 of them, the area on 16, the longest edge on 19, and
over the 46 the total edge length moves by 0.02 %. Cause, read in the
source of ELK 0.12.0: `RoutingDirector` builds the two candidate routes of
such a loop from `getSLPortsBySide().keySet()`, a `HashMap` keyed by the
`PortSide` enum, whose hash code is the identity hash of the JVM, different
on every launch; when both routes cost the same, the first one wins. Twelve
runs with `JAVA_TOOL_OPTIONS="-XX:+UnlockExperimentalVMOptions
-XX:hashCode=2"`, which gives every object the same identity hash, gave the
same result twelve times. **The bench sets that flag since the bar to beat
was recorded**: with it, two further full runs on this machine were
identical on all 27 493 files, as was a third in the container, and the
bar is the first of them. The flag picks a side on each of the 46 graphs,
29 of them as run 1 had it, 21 as run 2, 4 as neither; no other layout
changes.


## Limits of version 2

- **Rectangular nodes.** Any other shape is described by its bounding
  rectangle.
- **Routes in straight segments and cubic curves.** Other curves are
  described by cubic pieces.
- **No imposed node position.** Pinning a node's position will come with
  the layout engine.
- **About thirty nesting levels.** The JSON reader stops at 64 levels of
  nesting, and each container costs two; 31 levels read, 40 are refused.
- **One direction for the whole graph.** A container cannot have its own.
- **One source and one target per edge.** Hyperedges are not described.
- **No node labels.** A container keeps room for its title with
  `padding`; labels on nodes will come with the source format.

## Example

An equipment lending model: two entities, `MEMBRE` and `MATERIEL`, and
the association `PRET` that links them. Each edge carries a cardinality,
the label placed against the entity.

`gestpret.graph.json`:

```json
{
  "format": "foregram-graph",
  "version": 2,
  "name": "gestpret",
  "direction": "right",
  "nodes": [
    {"id": "MEMBRE", "width": 14000, "height": 15600},
    {"id": "MATERIEL", "width": 14000, "height": 15600},
    {"id": "PRET", "width": 18000, "height": 8800}
  ],
  "edges": [
    {"id": "membre-pret", "source": "MEMBRE", "target": "PRET", "labels": [{"width": 2600, "height": 1600, "at": "source"}]},
    {"id": "materiel-pret", "source": "MATERIEL", "target": "PRET", "labels": [{"width": 2600, "height": 1600, "at": "source"}]}
  ]
}
```

`gestpret.layout.json`, a layout routed by hand:

```json
{
  "format": "foregram-layout",
  "version": 2,
  "graph": "gestpret",
  "engine": {"name": "main", "version": "1", "options": "placement tracé à la main, pour les tests"},
  "nodes": [
    {"id": "MEMBRE", "x": 0, "y": 0},
    {"id": "MATERIEL", "x": 0, "y": 21600},
    {"id": "PRET", "x": 26000, "y": 14200}
  ],
  "edges": [
    {"id": "membre-pret", "route": [[14000, 7800], [20000, 7800], [20000, 16400], [26000, 16400]], "labels": [[14600, 5800]]},
    {"id": "materiel-pret", "route": [[14000, 29400], [20000, 29400], [20000, 20800], [26000, 20800]], "labels": [[14600, 27400]]}
  ]
}
```

A second example uses what version 2 adds: a container `System` with a
`padding`, two children, ports on `Client` and `Core` — one of them
pinned, with a box —, an edge from a child to its own container, and a
curved route.

`components.graph.json`:

```json
{
  "format": "foregram-graph",
  "version": 2,
  "name": "components",
  "direction": "right",
  "nodes": [
    {"id": "Client", "width": 10000, "height": 6000, "ports": [{"id": "req", "side": "east"}]},
    {"id": "System", "padding": 1000, "children": [
      {"id": "Core", "width": 12000, "height": 6000, "ports_ordered": true, "ports": [{"id": "in", "side": "west"}, {"id": "out", "side": "east", "x": 12000, "y": 3000, "width": 600, "height": 600}]},
      {"id": "Store", "width": 8000, "height": 5000}
    ]}
  ],
  "edges": [
    {"id": "e1", "source": "Client", "target": "Core", "source_port": "req", "target_port": "in", "labels": [{"width": 3000, "height": 1400, "at": "center"}]},
    {"id": "e2", "source": "Core", "target": "Store", "source_port": "out", "target_side": "west"},
    {"id": "e3", "source": "Store", "target": "System"}
  ]
}
```

`components.layout.json`:

```json
{
  "format": "foregram-layout",
  "version": 2,
  "graph": "components",
  "engine": {"name": "hand", "version": "1", "options": "routed by hand, for the tests"},
  "nodes": [
    {"id": "Client", "x": 0, "y": 2000, "ports": [{"id": "req", "x": 10000, "y": 3000}]},
    {"id": "System", "x": 16000, "y": 0, "width": 32000, "height": 10000},
    {"id": "Core", "x": 1000, "y": 1000, "ports": [{"id": "in", "x": 0, "y": 3000}, {"id": "out", "x": 12000, "y": 3000}]},
    {"id": "Store", "x": 19000, "y": 2000}
  ],
  "edges": [
    {"id": "e1", "route": [[10000, 5000], [13500, 5000], [13500, 4000], [17000, 4000]], "labels": [[11000, 3400]]},
    {"id": "e2", "route": [[29000, 4000], [35000, 4000]]},
    {"id": "e3", "route": [[39000, 7000], [[39000, 8500], [40000, 9000], [41000, 10000]]]}
  ]
}
```

All four files are in `crates/foregram-graph/tests/data/`, with two
examples of refusal.
