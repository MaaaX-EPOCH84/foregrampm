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
format diagrams will be written in: that one comes with the compiler.

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

## Validating

```
foregram-bench validate <graph.json> [<layout.json>...]
```

The graph is validated, then each layout is validated and confronted with
the graph. Every file is read, even after a refusal.

| Exit code | Meaning |
|---|---|
| 0 | Everything is valid |
| 1 | At least one file is refused |
| 2 | Wrong call, or unreadable file |

A refusal is written on the error output:

```
example.graph.json:10:5: error[G003]: edge "e1": `target` refers to node "B", which does not exist
```

that is the file, the line, the column, the code, the message. The
column counts characters, not bytes. The **code** is stable from one
version to the next and does not depend on the language of the message:
it is what a program must read.

The message is composed at display time from a per-language catalogue,
[`messages/en.txt`](../crates/foregram-graph/messages/en.txt): a refusal
carries its code and its arguments, not its sentence. English is the only
language for now; another one is a new catalogue, with the code unchanged.

A JSON syntax error stops the reading at the first one. Structure errors
are all reported at once, and so are meaning errors, but the latter are
only looked for in a file whose structure is right.

### Codes

JSON syntax:

| Code | Refusal |
|---|---|
| J001 | Text that is not UTF-8, or byte order mark |
| J002 | Unexpected character or word |
| J003 | Unexpected end of text, unterminated string |
| J004 | Non-integer number |
| J005 | Malformed number: leading zero, `-0`, lone `-` |
| J006 | Integer too large |
| J007 | String: invalid escape, control character |
| J008 | Duplicate key in an object |
| J009 | Nesting too deep |
| J010 | Content after the value |

Structure, for both formats:

| Code | Refusal |
|---|---|
| S001 | Unexpected type |
| S002 | Unknown key |
| S003 | Required key missing |
| S004 | Unexpected `format` |
| S005 | Unsupported `version` |
| S006 | Word outside the allowed list |
| S007 | Identifier empty, too long or with a control character |
| S008 | Integer outside its range |

Graph:

| Code | Refusal |
|---|---|
| G001 | Node identifier already used |
| G002 | Edge identifier already used |
| G003 | Edge to a node that does not exist |
| G004 | `order` without `layer` |
| G005 | Two nodes with the same `layer` and the same `order` |
| G006 | Port identifier already used |
| G007 | Edge to a port that does not exist, or that belongs to another node |
| G008 | A side and a port given on the same end of an edge |
| G009 | `ports_ordered` on a node whose ports lack a `side` |
| G010 | Port position without a side, off its side, or on a container |
| G011 | Size missing on a leaf, or given on a container |
| G012 | `padding` on a node without children |

Layout:

| Code | Refusal |
|---|---|
| L001 | The layout cites another graph |
| L002 | Node not in the graph |
| L003 | Node placed twice |
| L004 | Node not placed |
| L005 | Edge not in the graph |
| L006 | Edge routed twice |
| L007 | Edge without a route |
| L008 | Route with fewer than two points |
| L009 | Route end off the border of its node |
| L010 | Route end off the imposed side |
| L011 | Number of placed labels differs from the graph |
| L012 | Child outside its parent |
| L013 | Size missing on a placed container, or given on a placed leaf |
| L014 | Port not in the graph, not placed, or placed twice |
| L015 | Port anchor off the border, off its side, or off its fixed position |
| L016 | Route end off its port |
| L017 | Route starting with a curve |

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
| X001 | Text that is not UTF-8 |
| X002 | Malformed XML, with the parser's own detail |
| X003 | Root element other than `graphml` |
| X004 | Not exactly one `graph` in `graphml` |
| X005 | Element not supported, or text where none is expected |
| X006 | Attribute not supported |
| X007 | `id` of a node or an edge, `source` or `target` missing |
| X008 | `edgedefault` or `directed` with an invalid value |

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
| E001 | Text that is not UTF-8 |
| E002 | Unterminated string or comment |
| E003 | Unexpected character |
| E004 | Unexpected token or end of text, with what was expected |
| E005 | Unknown escape sequence in a string |
| E006 | A `layout` key given twice |

ELK import:

| Code | Refusal |
|---|---|
| E007 | Edge end that refers to no node or port |
| E008 | Hyperedge: more than one source or one target |
| E009 | Number that cannot be carried in hundredths of a pixel |
| E010 | Option value outside the words the import knows |
| E011 | Edge end on the root graph itself, or ports on the root |

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
| M001 | Text that is not UTF-8 |
| M002 | Line in column one other than `entite NAME` or `association NAME` |
| M003 | Indented line before any block |
| M004 | Link inside an entity |
| M005 | Indented line that is neither a property nor a link |
| M006 | Entity or association declared twice |
| M007 | Link to something that is not a declared entity |
| M008 | Association with fewer than two links |

## Reference engines

The benchmark measures its own layout engine against two references, run at
pinned versions and brought back into the layout format: `dot`, of Graphviz,
and ELK Layered. `corpus/fetch.sh` installs both in the cache from
`corpus/manifest.txt` — the Graphviz Debian package of the pinned release,
unpacked without installing; the ELK jars from Maven Central, each with its
SHA-256, and the benchmark's small Java driver compiled against them with
`javac`. Running ELK needs a Java runtime on the `PATH`; nothing else is
installed on the machine.

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
| R001 | Engine not found in the cache |
| R002 | Engine version differs from the manifest |
| R003 | Engine failed on the graph, with its message |
| R004 | Engine timed out (60 s) |
| R005 | Engine output not understood |
| R006 | Number in the output that cannot be carried in hundredths of a pixel |
| R007 | ELK changed the size of a leaf |

## Metrics

The benchmark compares engines on numbers read off a layout confronted with
its graph. The layout must pass the validation above; the metrics then
measure what validation does not judge.

```
foregram-bench metrics <graph> <layout.json>...
```

The command checks each layout against the graph, which may be a
`.graph.json`, `.graphml`, `.elkt` or `.mcd` file, and writes one line per
measure. A layout that fails the check is refused, not measured. All
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
lives in the `foregram-render` crate; the benchmark exposes it:

```
foregram-bench render <graph> <layout.json> <out.svg|out.png> [--scale <n>]
```

The command checks the layout against the graph, which may be a
`.graph.json`, `.graphml`, `.elkt` or `.mcd` file, and writes the drawing
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
| port | A dark square of 4 px centred on the anchor, or the port's own box when the graph gives one |

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
| V001 | Scale outside 1 to 8 |
| V002 | Image larger than 50,000,000 pixels |
| V003 | PNG encoder failure |

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
