// ForegramPM — banc de mesure du placement : import GraphML (Rome, North).
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Lit le GraphML des corpus Rome et North et en fait un [`Graph`] au
//! format d'échange, en mémoire.
//!
//! N'est accepté que ce que le format sait dire : un `graphml` qui contient
//! un seul `graph`, des `node` et des `edge` sans contenu. Tout le reste
//! (`data`, `port`, `hyperedge`, graphe imbriqué, attribut inconnu) est
//! refusé plutôt qu'ignoré : un graphe amputé en silence fausserait le banc.
//!
//! Le GraphML ne donne pas de taille : chaque nœud reçoit [`NODE_SIZE`]. Un
//! lien va de `source` à `target` tels qu'écrits, qu'il soit orienté ou non :
//! les liens non orientés de Rome gardent ainsi l'ordre du fichier.
//!
//! Les refus de lecture portent la famille `X` ; ceux du graphe produit sont
//! ceux du validateur, avec la position de l'élément dans le `.graphml`.

use foregram_graph::{Diagnostic, Edge, Graph, Message, Node, Pos, Size};
use roxmltree::{Document, ParsingOptions};

/// Taille commune à tous les nœuds importés : rapport exact de l'A4, 99/70,
/// en paysage, soit 49,5 × 35 px, au plus près du nœud par défaut de
/// Graphviz (54 × 36). Notée aussi dans `corpus/manifest.txt`.
pub const NODE_SIZE: Size = Size {
    width: 4950,
    height: 3500,
};

const GRAPHML_NS: &str = "http://graphml.graphdrawing.org/xmlns";
/// `xsi:schemaLocation` et ses voisins ne disent rien du graphe.
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Lit un fichier GraphML. `name` devient le nom du graphe.
///
/// Rend le graphe, ou tous les refus relevés. Un défaut de syntaxe XML
/// arrête la lecture ; les défauts de structure sont tous rendus ; le graphe
/// n'est validé que si sa structure est juste.
pub fn import(name: &str, bytes: &[u8], size: Size) -> Result<Graph, Vec<Diagnostic>> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid = bytes.get(..error.valid_up_to()).unwrap_or_default();
            let prefix = std::str::from_utf8(valid).unwrap_or_default();
            let pos = Lines::new(prefix).pos(prefix.len());
            return Err(vec![Diagnostic::new(pos, Message::new("X001"))]);
        }
    };
    // roxmltree saute la marque d'ordre des octets mais compte ses positions
    // depuis le début du texte : l'ôter ici garde des colonnes justes.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let options = ParsingOptions {
        // Chaque fichier de Rome et North déclare une DTD extérieure ;
        // roxmltree ne la va jamais chercher.
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = match Document::parse_with_options(text, options) {
        Ok(doc) => doc,
        Err(error) => {
            let pos = match error {
                // Ces erreurs-là n'ont pas de position : roxmltree rend 1:1.
                roxmltree::Error::NoRootNode
                | roxmltree::Error::UnclosedRootNode
                | roxmltree::Error::UnexpectedEndOfStream
                | roxmltree::Error::NodesLimitReached
                | roxmltree::Error::AttributesLimitReached
                | roxmltree::Error::NamespacesLimitReached
                | roxmltree::Error::DtdDetected => Pos::NONE,
                _ => Pos {
                    line: error.pos().row,
                    col: error.pos().col,
                },
            };
            let message = Message::new("X002").text("detail", error);
            return Err(vec![Diagnostic::new(pos, message)]);
        }
    };

    let mut reader = Reader {
        lines: Lines::new(text),
        diags: Vec::new(),
    };
    let graph = reader.graphml(&doc, name, size);
    let mut diags = reader.diags;
    if let Some(graph) = graph
        && diags.is_empty()
    {
        diags = graph.validate();
        if diags.is_empty() {
            return Ok(graph);
        }
    }
    // Même ordre que la bibliothèque : par position, puis par code.
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
    Err(diags)
}

struct Reader<'t> {
    lines: Lines<'t>,
    diags: Vec<Diagnostic>,
}

type XmlNode<'a, 'input> = roxmltree::Node<'a, 'input>;

impl Reader<'_> {
    fn graphml(&mut self, doc: &Document, name: &str, size: Size) -> Option<Graph> {
        let root = doc.root_element();
        if !is_graphml(root, "graphml") {
            let message = Message::new("X003").text("name", root.tag_name().name());
            self.refuse(root.range().start, message);
            return None;
        }
        self.attributes(root, &[]);
        let mut graphs = Vec::new();
        for child in self.children(root) {
            if is_graphml(child, "graph") {
                graphs.push(child);
            } else {
                self.unsupported(child, root);
            }
        }
        let [graph] = graphs[..] else {
            let at = graphs.get(1).map_or(root, |second| *second);
            let message = Message::new("X004").text("count", graphs.len());
            self.refuse(at.range().start, message);
            return None;
        };

        self.attributes(graph, &["id", "edgedefault"]);
        if let Some(value) = graph.attribute("edgedefault")
            && value != "directed"
            && value != "undirected"
        {
            self.bad_value(graph, "edgedefault", "X008.edgedefault", value);
        }
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for child in self.children(graph) {
            if is_graphml(child, "node") {
                self.attributes(child, &["id"]);
                self.empty(child);
                if let Some(id) = self.required(child, "id") {
                    nodes.push(Node {
                        id: id.to_string(),
                        size: Some(size),
                        layer: None,
                        order: None,
                        padding: None,
                        ports_ordered: false,
                        ports: Vec::new(),
                        children: Vec::new(),
                        pos: self.pos(child.range().start),
                    });
                }
            } else if is_graphml(child, "edge") {
                self.attributes(child, &["id", "source", "target", "directed"]);
                self.empty(child);
                if let Some(value) = child.attribute("directed")
                    && value != "true"
                    && value != "false"
                {
                    self.bad_value(child, "directed", "X008.directed", value);
                }
                let id = self.required(child, "id");
                let source = self.required(child, "source");
                let target = self.required(child, "target");
                if let (Some(id), Some(source), Some(target)) = (id, source, target) {
                    edges.push(Edge {
                        id: id.to_string(),
                        source: source.to_string(),
                        target: target.to_string(),
                        source_port: None,
                        target_port: None,
                        source_side: None,
                        target_side: None,
                        labels: Vec::new(),
                        pos: self.pos(child.range().start),
                    });
                }
            } else {
                self.unsupported(child, graph);
            }
        }
        Some(Graph {
            name: name.to_string(),
            direction: None,
            nodes,
            edges,
            pos: self.pos(graph.range().start),
        })
    }

    /// Les éléments enfants. Un texte autre que des blancs est refusé ;
    /// commentaires et instructions de traitement sont sans effet.
    fn children<'a, 'input>(&mut self, parent: XmlNode<'a, 'input>) -> Vec<XmlNode<'a, 'input>> {
        let mut out = Vec::new();
        for child in parent.children() {
            if child.is_element() {
                out.push(child);
            } else if child.is_text() && !child.text().unwrap_or("").trim().is_empty() {
                let message = Message::new("X005.text").text("parent", parent.tag_name().name());
                self.refuse(child.range().start, message);
            }
        }
        out
    }

    /// Un `node` ou un `edge` n'a aucun contenu : ni `data`, ni `port`, ni graphe imbriqué.
    fn empty(&mut self, element: XmlNode) {
        for child in self.children(element) {
            self.unsupported(child, element);
        }
    }

    fn unsupported(&mut self, child: XmlNode, parent: XmlNode) {
        let message = Message::new("X005")
            .text("name", child.tag_name().name())
            .text("parent", parent.tag_name().name());
        self.refuse(child.range().start, message);
    }

    /// Refuse tout attribut hors de `known`, sauf ceux de l'espace `xsi`.
    fn attributes(&mut self, element: XmlNode, known: &[&str]) {
        for attribute in element.attributes() {
            let ours = attribute.namespace().is_none() && known.contains(&attribute.name());
            if ours || attribute.namespace() == Some(XSI_NS) {
                continue;
            }
            let message = Message::new("X006")
                .text("name", attribute.name())
                .text("element", element.tag_name().name());
            self.refuse(attribute.range().start, message);
        }
    }

    fn required<'a>(&mut self, element: XmlNode<'a, '_>, name: &'static str) -> Option<&'a str> {
        let value = element.attribute(name);
        if value.is_none() {
            let message = Message::new("X007")
                .text("element", element.tag_name().name())
                .text("name", name);
            self.refuse(element.range().start, message);
        }
        value
    }

    fn bad_value(&mut self, element: XmlNode, name: &str, key: &'static str, value: &str) {
        let start = element
            .attributes()
            .find(|a| a.name() == name)
            .map_or(element.range().start, |a| a.range().start);
        self.refuse(start, Message::new(key).text("value", value));
    }

    fn refuse(&mut self, offset: usize, message: Message) {
        let pos = self.pos(offset);
        self.diags.push(Diagnostic::new(pos, message));
    }

    fn pos(&self, offset: usize) -> Pos {
        self.lines.pos(offset)
    }
}

/// Un élément de ce nom, sans espace de noms ou dans celui de GraphML.
fn is_graphml(node: XmlNode, name: &str) -> bool {
    let tag = node.tag_name();
    tag.name() == name && matches!(tag.namespace(), None | Some(GRAPHML_NS))
}

/// Début de chaque ligne d'un texte, pour passer d'un décalage en octets à
/// une ligne et une colonne en points de code. roxmltree sait le faire, mais
/// en relisant le texte depuis le début à chaque appel.
struct Lines<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Lines { text, starts }
    }

    fn pos(&self, offset: usize) -> Pos {
        let line = self.starts.partition_point(|&start| start <= offset);
        let start = self.starts.get(line.wrapping_sub(1)).copied().unwrap_or(0);
        let col = self
            .text
            .get(start..offset)
            .map_or(0, |s| s.chars().count());
        Pos {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            col: u32::try_from(col + 1).unwrap_or(u32::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size {
        width: 10,
        height: 20,
    };

    fn refusals(text: &str) -> Vec<(&'static str, u32, u32)> {
        import("g", text.as_bytes(), SIZE)
            .unwrap_err()
            .iter()
            .map(|d| (d.code, d.pos.line, d.pos.col))
            .collect()
    }

    fn texts(text: &str) -> Vec<String> {
        import("g", text.as_bytes(), SIZE)
            .unwrap_err()
            .iter()
            .map(Diagnostic::to_string)
            .collect()
    }

    /// Comme dans Rome : DTD extérieure, commentaire, liens non orientés.
    const ROME: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <!-- This file was written by the internal XML-Handler of Y-Files.-->\n\
        <!DOCTYPE graphml SYSTEM \"http://www.graphdrawing.org/dtds/graphml.dtd\">\n\
        <graphml>\n\
        <graph edgedefault=\"undirected\" id=\"G\">\n\
        <node id=\"n0\"/>\n\
        <node id=\"n1\"/>\n\
        <node id=\"n2\"/>\n\
        <edge id=\"e0\" source=\"n2\" target=\"n0\"/>\n\
        <edge id=\"e1\" source=\"n0\" target=\"n1\"/>\n\
        </graph>\n\
        </graphml>\n";

    #[test]
    fn a_rome_graph_is_read_in_file_order_with_the_given_size() {
        let graph = import("grafo1.10", ROME.as_bytes(), SIZE).unwrap();
        assert_eq!(graph.name, "grafo1.10");
        assert_eq!(graph.direction, None);
        let nodes: Vec<(&str, Option<Size>)> = graph
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.size))
            .collect();
        assert_eq!(
            nodes,
            [("n0", Some(SIZE)), ("n1", Some(SIZE)), ("n2", Some(SIZE))]
        );
        let edges: Vec<(&str, &str, &str)> = graph
            .edges
            .iter()
            .map(|e| (e.id.as_str(), e.source.as_str(), e.target.as_str()))
            .collect();
        assert_eq!(edges, [("e0", "n2", "n0"), ("e1", "n0", "n1")]);
        assert_eq!(graph.nodes[2].pos, Pos { line: 8, col: 1 });
        assert_eq!(graph.edges[0].pos, Pos { line: 9, col: 1 });
    }

    #[test]
    fn a_north_graph_without_edgedefault_is_read_the_same_way() {
        let text = ROME.replace(" edgedefault=\"undirected\"", "");
        let graph = import("g.10.1", text.as_bytes(), SIZE).unwrap();
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn the_graphml_namespace_xsi_comments_and_a_byte_order_mark_are_accepted() {
        let text = "\u{feff}<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\" \
            xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
            xsi:schemaLocation=\"http://graphml.graphdrawing.org/xmlns x.xsd\">\
            <?yfiles x?><graph edgedefault=\"directed\"><!-- c --><node id=\"a\"/>\
            <edge id=\"e\" source=\"a\" target=\"a\" directed=\"false\"/></graph></graphml>";
        let graph = import("g", text.as_bytes(), SIZE).unwrap();
        assert_eq!(graph.edges[0].source, "a");
        // Après la marque, la première colonne reste la première.
        let bad = "\u{feff}<graph/>";
        assert_eq!(refusals(bad), [("X003", 1, 1)]);
    }

    #[test]
    fn imported_graphs_come_out_canonical_and_identical() {
        let once = import("g", ROME.as_bytes(), NODE_SIZE).unwrap().to_json();
        let twice = import("g", ROME.as_bytes(), NODE_SIZE).unwrap().to_json();
        assert_eq!(once, twice);
        assert_eq!(Graph::parse(once.as_bytes()).unwrap().to_json(), once);
        assert!(once.contains("{\"id\": \"n0\", \"width\": 4950, \"height\": 3500}"));
    }

    #[test]
    fn text_that_is_not_utf8_is_refused_where_it_breaks() {
        let mut bytes = b"<graphml>\n  <graph>\xff".to_vec();
        bytes.extend_from_slice(b"</graph></graphml>");
        let diags = import("g", &bytes, SIZE).unwrap_err();
        assert_eq!(
            (diags[0].code, diags[0].pos),
            ("X001", Pos { line: 2, col: 10 })
        );
    }

    #[test]
    fn malformed_xml_is_refused_with_the_parser_detail() {
        assert_eq!(refusals("<graphml>\n <graph></graphml>"), [("X002", 2, 9)]);
        assert_eq!(
            texts("<graphml>\n <graph></graphml>"),
            ["2:9: error[X002]: malformed XML: expected 'graph' tag, not 'graphml' at 2:9"]
        );
        // Sans position véritable, le refus n'en invente pas.
        let diags = import("g", b"", SIZE).unwrap_err();
        assert_eq!((diags[0].code, diags[0].pos), ("X002", Pos::NONE));
    }

    #[test]
    fn the_root_must_be_graphml_holding_one_graph() {
        assert_eq!(refusals("<gml/>"), [("X003", 1, 1)]);
        assert_eq!(
            refusals("<graphml xmlns=\"urn:other\"><graph/></graphml>"),
            [("X003", 1, 1)]
        );
        assert_eq!(refusals("<graphml>\n</graphml>"), [("X004", 1, 1)]);
        assert_eq!(
            refusals("<graphml><graph/>\n<graph/></graphml>"),
            [("X004", 2, 1)]
        );
        assert_eq!(
            texts("<graphml/>"),
            ["1:1: error[X004]: `graphml` must contain exactly one `graph`, it contains 0"]
        );
    }

    #[test]
    fn what_the_format_cannot_say_is_refused_not_dropped() {
        let text = "<graphml><key id=\"d0\"/><graph>\n\
            <node id=\"a\"><data key=\"d0\">x</data></node>\n\
            <node id=\"b\"><port name=\"p\"/><graph/></node>\n\
            <hyperedge/>\n\
            loose\n\
            </graph></graphml>";
        assert_eq!(
            refusals(text),
            [
                ("X005", 1, 10),
                ("X005", 2, 14),
                ("X005", 3, 14),
                ("X005", 3, 30),
                ("X005", 4, 1),
                ("X005", 4, 13),
            ]
        );
        let texts = texts(text);
        assert_eq!(
            texts[0],
            "1:10: error[X005]: element `key` not supported inside `graphml`"
        );
        assert_eq!(
            texts[3],
            "3:30: error[X005]: element `graph` not supported inside `node`"
        );
        assert_eq!(
            texts[5],
            "4:13: error[X005]: text not expected inside `graph`"
        );
    }

    #[test]
    fn unknown_attributes_are_refused() {
        let text = "<graphml version=\"1\"><graph parse.order=\"free\">\n\
            <node id=\"a\" label=\"A\"/>\n\
            <edge id=\"e\" source=\"a\" target=\"a\" sourceport=\"p\"/>\n\
            </graph></graphml>";
        assert_eq!(
            refusals(text),
            [
                ("X006", 1, 10),
                ("X006", 1, 29),
                ("X006", 2, 14),
                ("X006", 3, 36)
            ]
        );
        assert_eq!(
            texts(text)[3],
            "3:36: error[X006]: attribute `sourceport` not supported on `edge`"
        );
    }

    #[test]
    fn identifiers_and_ends_are_required() {
        let text = "<graphml><graph>\n<node/>\n<edge source=\"a\"/>\n</graph></graphml>";
        assert_eq!(
            refusals(text),
            [("X007", 2, 1), ("X007", 3, 1), ("X007", 3, 1)]
        );
        assert_eq!(
            texts(text)[0],
            "2:1: error[X007]: `node` without its `id` attribute"
        );
    }

    #[test]
    fn edge_orientation_values_are_checked() {
        let text = "<graphml><graph edgedefault=\"mixed\">\n\
            <node id=\"a\"/><edge id=\"e\" source=\"a\" target=\"a\" directed=\"yes\"/>\n\
            </graph></graphml>";
        assert_eq!(refusals(text), [("X008", 1, 17), ("X008", 2, 50)]);
        assert_eq!(
            texts(text),
            [
                "1:17: error[X008]: `edgedefault` is \"mixed\": `directed` or `undirected` expected",
                "2:50: error[X008]: `directed` is \"yes\": `true` or `false` expected",
            ]
        );
    }

    #[test]
    fn the_validator_refuses_the_rest_with_graphml_positions() {
        let text = "<graphml><graph>\n<node id=\"a\"/>\n<node id=\"a\"/>\n\
            <edge id=\"e\" source=\"a\" target=\"z\"/>\n</graph></graphml>";
        assert_eq!(refusals(text), [("G001", 3, 1), ("G003", 4, 1)]);
        assert_eq!(
            texts(text)[1],
            "4:1: error[G003]: edge \"e\": `target` refers to node \"z\", which does not exist"
        );
    }

    #[test]
    fn the_manifest_records_the_size_given_to_every_node() {
        let path = format!("{}/../../corpus/manifest.txt", env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(path).unwrap();
        let line = format!("node_size = {} x {}", NODE_SIZE.width, NODE_SIZE.height);
        for corpus in ["[rome]", "[north]"] {
            let section = manifest
                .split(corpus)
                .nth(1)
                .unwrap()
                .split("\n[")
                .next()
                .unwrap();
            assert!(
                section.lines().any(|l| l == line),
                "{corpus} : {line} absent"
            );
        }
        // Le rapport exact de l'A4, 297/210 = 99/70, en paysage.
        assert_eq!(NODE_SIZE.width * 70, NODE_SIZE.height * 99);
    }

    #[test]
    fn line_starts_turn_offsets_into_code_point_columns() {
        let lines = Lines::new("é\nab\n");
        assert_eq!(lines.pos(0), Pos { line: 1, col: 1 });
        assert_eq!(lines.pos(2), Pos { line: 1, col: 2 });
        assert_eq!(lines.pos(3), Pos { line: 2, col: 1 });
        assert_eq!(lines.pos(5), Pos { line: 2, col: 3 });
        assert_eq!(lines.pos(6), Pos { line: 3, col: 1 });
    }
}
