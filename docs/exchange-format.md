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
