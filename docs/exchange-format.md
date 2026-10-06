# Exchange format of the layout benchmark

Two JSON files describe one benchmark case:

- the **graph** (`foregram-graph`): what a layout engine receives —
  rectangular nodes, edges, their labels, and whatever the author chose
  to pin down;
- the **layout** (`foregram-layout`): what an engine returns for that
  graph — the position of every node, the route of every edge, the
  position of every label.

The layout repeats nothing from the graph. An engine therefore cannot
alter a size or an edge; it can only forget an element or cite one that
does not exist, and the validator reports it.

By convention a graph is named `<name>.graph.json` and its layouts
`<name>.<engine>.layout.json`.

This format serves the benchmark and the layout engine. It is not the
format diagrams will be written in: that one comes with the compiler.

## Units and axes

- Every length is an **integer**, in **hundredths of a pixel**: a node
  120 pixels wide has `"width": 12000`.
- The x axis points right, the y axis **points down**.
- The position of a rectangle, node or label, is that of its top-left
  corner.
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
| `version` | integer | required | `1` |
| `name` | identifier | required | Name of the graph; the layout cites it |
| `direction` | word | optional | Direction in which layers follow each other: `right`, `down`, `left` or `up` |
| `nodes` | array | required | The nodes |
| `edges` | array | required | The edges |

A **node**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | Unique among nodes |
| `width`, `height` | integers | required | Dimensions of the rectangle |
| `layer` | integer | optional | Imposed layer, from 0 to 1,000,000 |
| `order` | integer | optional | Imposed rank within the layer; requires `layer` |

An **edge**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | Unique among edges |
| `source`, `target` | identifiers | required | Start node, end node |
| `source_side`, `target_side` | words | optional | Imposed side of the node: `north`, `south`, `east` or `west` |
| `labels` | array | optional | The labels of the edge |

A **label**:

| Key | Type | | Meaning |
|---|---|---|---|
| `width`, `height` | integers | required | Dimensions of the rectangle |
| `at` | word | required | End the label relates to: `source`, `center` or `target` |

Rules:

- An **identifier** is a non-empty string, without control characters,
  of at most 255 characters.
- Node identifiers and edge identifiers are two separate spaces: a node
  and an edge may carry the same one.
- `source` and `target` refer to nodes that exist.
- A **loop** (an edge from a node to itself) is allowed, as are several
  edges between the same two nodes.
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

The validator checks imposed sides, because they can be read off the
route. It does not check `layer` or `order`: whether an engine honoured
them is a benchmark measurement, not a validity condition.

## The layout

| Key | Type | | Meaning |
|---|---|---|---|
| `format` | string | required | `"foregram-layout"` |
| `version` | integer | required | `1` |
| `graph` | identifier | required | The `name` of the graph laid out |
| `engine` | object | required | The engine that produced the layout |
| `nodes` | array | required | The position of every node |
| `edges` | array | required | The route of every edge |

The **engine**: `name` and `version`, required, and `options`, optional,
which says with which settings it ran. Enough to replay the layout.

A **placed node**: `id`, `x`, `y`, all required.

A **routed edge**:

| Key | Type | | Meaning |
|---|---|---|---|
| `id` | identifier | required | The edge of the graph |
| `route` | array of points | required | Polyline from start to end; at least two points |
| `labels` | array of points | optional | Position of every label, in the order of the graph |

A **point** is written `[x, y]`.

Rules, checked by confronting the layout with its graph:

- every node of the graph is placed once, and no other;
- every edge of the graph is routed once, and no other;
- the first point of a route lies **on the border** of the start node,
  the last on the border of the end node, and on the imposed side if
  there is one;
- an edge has as many placed labels as the graph gives it.

Overlapping nodes or crossing edges do not make a layout invalid: that is
what the benchmark measures.

## Canonical form

The library always writes a file the same way: keys in the order of the
tables above, one node and one edge per line, two-space indentation,
optional keys omitted when they have no value, a final newline. Inside a
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

## Limits of version 1

- **Flat graphs.** A node contains no other node.
- **Rectangular nodes.** Any other shape is described by its bounding
  rectangle.
- **Polyline routes.** A curve is described by the points that
  approximate it.
- **No imposed position.** Pinning a node's position will come with the
  layout engine.

## Example

An equipment lending model: two entities, `MEMBRE` and `MATERIEL`, and
the association `PRET` that links them. Each edge carries a cardinality,
the label placed against the entity.

`gestpret.graph.json`:

```json
{
  "format": "foregram-graph",
  "version": 1,
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
  "version": 1,
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

Both files are in `crates/foregram-graph/tests/data/`, with two examples
of refusal.
