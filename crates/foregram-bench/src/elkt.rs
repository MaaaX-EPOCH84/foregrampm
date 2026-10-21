// ForegramPM — placement measurement bench: reader for ELK's text format (.elkt).
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads an `.elkt` file, ELK's text format, and returns the tree it
//! describes, as written: nothing is interpreted, nothing is lost. It is
//! `elk.rs` that then derives an exchange-format graph from it.
//!
//! The reader follows the official grammar `ElkGraph.xtext` of the
//! `eclipse-elk/elk` repository (`plugins/org.eclipse.elk.graph.text`, commit
//! `60b6846` of 2026-07-09), and the Xtext terminals it imports:
//!
//! ```text
//! RootNode : ('graph' ID)? Layout? Property* (Label | Port | Node | Edge)*
//! Node     : 'node' ID ('{' Layout? Property* (Label | Port | Node | Edge)* '}')?
//! Port     : 'port' ID ('{' Layout? Property* Label* '}')?
//! Label    : 'label' (ID ':')? STRING ('{' Layout? Property* Label* '}')?
//! Edge     : 'edge' (ID ':')? QualifiedId (',' QualifiedId)* '->'
//!            QualifiedId (',' QualifiedId)* ('{' EdgeLayout? Property* Label* '}')?
//! Layout   : 'layout' '[' (('position' ':' Number ',' Number)?
//!                        & ('size' ':' Number ',' Number)?) ']'
//! EdgeLayout : 'layout' '[' (SingleSection | Section+) ']'
//! Section  : 'section' ID ('->' ID (',' ID)*)? '[' SectionBody ']'
//! SingleSection : SectionBody
//! SectionBody : (('incoming' ':' QualifiedId)? & ('outgoing' ':' QualifiedId)?
//!             & ('start' ':' Number ',' Number)? & ('end' ':' Number ',' Number)?)
//!               ('bends' ':' Number ',' Number ('|' Number ',' Number)*)? Property*
//! Property : PropertyKey ':' (STRING | QualifiedId | Number | 'true' | 'false' | 'null')
//! PropertyKey : ID ('.' ID)*       -- no whitespace between segments
//! QualifiedId : ID ('.' ID)*
//! ID     : '^'? [a-zA-Z_] [a-zA-Z_0-9]*   -- the '^' escapes a keyword, and is dropped
//! STRING : '"' ('\' . | [^\"])* '"' | "'" ('\' . | [^\'])* "'"
//! Number : ('+' | '-')? INT ('.' INT)? (('e' | 'E') ('+' | '-')? INT)?
//! ```
//!
//! Whitespace (spaces, tabs, line breaks) and the comments `/* */` and
//! `//` separate tokens. An `&` group admits each member at most once,
//! in any order. A `label` is a separate element in ELK's model; its
//! `position` and `size` are read like those of a node.
//!
//! Numbers stay as text: the import interprets them. A string is returned
//! without its quotes or escapes, Java-style, as ELK does for a label's
//! text.
//!
//! A syntax error stops the reading at the first one, family `E`, with its
//! position in the `.elkt`.

use foregram_graph::{Diagnostic, Message, Pos};

use crate::lines::Lines;

/// A node: the file's root, or a `node`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElkNode {
    /// `None` for a root without `graph <id>`.
    pub id: Option<String>,
    pub layout: Shape,
    pub properties: Vec<Property>,
    pub labels: Vec<ElkLabel>,
    pub ports: Vec<ElkPort>,
    pub children: Vec<ElkNode>,
    pub edges: Vec<ElkEdge>,
    pub pos: Pos,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElkPort {
    pub id: String,
    pub layout: Shape,
    pub properties: Vec<Property>,
    pub labels: Vec<ElkLabel>,
    pub pos: Pos,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElkLabel {
    pub id: Option<String>,
    pub text: String,
    pub layout: Shape,
    pub properties: Vec<Property>,
    pub labels: Vec<ElkLabel>,
    pub pos: Pos,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElkEdge {
    pub id: Option<String>,
    pub sources: Vec<QualifiedId>,
    pub targets: Vec<QualifiedId>,
    pub sections: Vec<Section>,
    pub properties: Vec<Property>,
    pub labels: Vec<ElkLabel>,
    pub pos: Pos,
}

/// The `layout [...]` of a node, a port or a label.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shape {
    pub position: Option<(Number, Number)>,
    pub size: Option<(Number, Number)>,
}

/// A section of an edge's route. The single section of a `layout` without
/// `section` has no identifier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Section {
    pub id: Option<String>,
    pub outgoing_sections: Vec<String>,
    pub incoming: Option<QualifiedId>,
    pub outgoing: Option<QualifiedId>,
    pub start: Option<(Number, Number)>,
    pub end: Option<(Number, Number)>,
    pub bends: Vec<(Number, Number)>,
    pub properties: Vec<Property>,
    pub pos: Pos,
}

/// A property: its key as written, and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub key: String,
    pub value: Value,
    pub pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A string, without its quotes.
    Str(String),
    /// An identifier, possibly qualified: `FIXED_SIDE`, `a.b`.
    Id(String),
    Number(Number),
    Bool(bool),
    Null,
}

/// A number, kept as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Number {
    pub text: String,
    pub pos: Pos,
}

/// A qualified name: `parent.child.port`, one segment per `ID`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedId {
    pub segments: Vec<String>,
    pub pos: Pos,
}

impl QualifiedId {
    pub fn text(&self) -> String {
        self.segments.join(".")
    }
}

/// Reads an `.elkt` file. Returns the root, or the first syntax error.
pub fn parse(bytes: &[u8]) -> Result<ElkNode, Diagnostic> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid = bytes.get(..error.valid_up_to()).unwrap_or_default();
            let prefix = std::str::from_utf8(valid).unwrap_or_default();
            let pos = Lines::new(prefix).pos(prefix.len());
            return Err(Diagnostic::new(pos, Message::new("E001")));
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines = Lines::new(text);
    let tokens = match lex(text) {
        Ok(tokens) => tokens,
        Err((offset, message)) => return Err(Diagnostic::new(lines.pos(offset), message)),
    };
    let mut parser = Parser {
        text,
        lines,
        tokens,
        at: 0,
    };
    let root = parser.root();
    root.map_err(|(offset, message)| Diagnostic::new(parser.lines.pos(offset), message))
}

// ------------------------------------------------------------------ tokens

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tok {
    /// A word: identifier or keyword. `escaped` is set for `^word`, which
    /// is always an identifier.
    Word {
        escaped: bool,
    },
    Str,
    Number,
    /// `{`, `}`, `[`, `]`, `:`, `,`, `.`, `|`
    Punct(char),
    Arrow,
    End,
}

#[derive(Clone, Copy, Debug)]
struct Token {
    tok: Tok,
    start: usize,
    end: usize,
}

const KEYWORDS: [&str; 17] = [
    "graph", "node", "port", "label", "edge", "layout", "position", "size", "section", "incoming",
    "outgoing", "start", "end", "bends", "true", "false", "null",
];

type Fail = (usize, Message);

/// Splits the text into tokens. The error is returned with its byte offset.
fn lex(text: &str) -> Result<Vec<Token>, Fail> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        let start = i;
        let tok = match b {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while let Some(&c) = bytes.get(i) {
                    if c == b'\n' {
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let Some(close) = text.get(i + 2..).and_then(|rest| rest.find("*/")) else {
                    return Err((start, Message::new("E002.comment")));
                };
                i += 2 + close + 2;
                continue;
            }
            b'"' | b'\'' => {
                i += 1;
                loop {
                    match bytes.get(i) {
                        None => return Err((start, Message::new("E002.string"))),
                        Some(&c) if c == b => break,
                        Some(b'\\') => i += 2,
                        Some(_) => i += 1,
                    }
                }
                i += 1;
                Tok::Str
            }
            b'^' | b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let escaped = b == b'^';
                if escaped {
                    i += 1;
                    if !bytes
                        .get(i)
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                    {
                        return Err((start, Message::new("E003").text("char", '^')));
                    }
                }
                while bytes
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    i += 1;
                }
                Tok::Word { escaped }
            }
            b'-' if bytes.get(i + 1) == Some(&b'>') => {
                i += 2;
                Tok::Arrow
            }
            b'0'..=b'9' | b'+' | b'-' => {
                if !b.is_ascii_digit() {
                    i += 1;
                }
                if !bytes.get(i).is_some_and(u8::is_ascii_digit) {
                    let c = text
                        .get(start..)
                        .and_then(|s| s.chars().next())
                        .unwrap_or('?');
                    return Err((start, Message::new("E003").text("char", c)));
                }
                i = digits(bytes, i);
                if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    i = digits(bytes, i + 1);
                }
                if matches!(bytes.get(i), Some(b'e' | b'E')) {
                    let mut j = i + 1;
                    if matches!(bytes.get(j), Some(b'+' | b'-')) {
                        j += 1;
                    }
                    if bytes.get(j).is_some_and(u8::is_ascii_digit) {
                        i = digits(bytes, j);
                    }
                }
                Tok::Number
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' | b'.' | b'|' => {
                i += 1;
                Tok::Punct(b as char)
            }
            _ => {
                let c = text
                    .get(start..)
                    .and_then(|s| s.chars().next())
                    .unwrap_or('?');
                return Err((start, Message::new("E003").text("char", c)));
            }
        };
        tokens.push(Token { tok, start, end: i });
    }
    tokens.push(Token {
        tok: Tok::End,
        start: bytes.len(),
        end: bytes.len(),
    });
    Ok(tokens)
}

fn digits(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    i
}

// ------------------------------------------------------------------ parsing

struct Parser<'t> {
    text: &'t str,
    lines: Lines<'t>,
    tokens: Vec<Token>,
    at: usize,
}

/// What an element of a `layout` `&` group can be.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Position,
    Size,
    Incoming,
    Outgoing,
    Start,
    End,
}

impl Slot {
    fn keyword(self) -> &'static str {
        match self {
            Slot::Position => "position",
            Slot::Size => "size",
            Slot::Incoming => "incoming",
            Slot::Outgoing => "outgoing",
            Slot::Start => "start",
            Slot::End => "end",
        }
    }
}

impl<'t> Parser<'t> {
    fn peek(&self) -> Token {
        self.tokens.get(self.at).copied().unwrap_or(Token {
            tok: Tok::End,
            start: self.text.len(),
            end: self.text.len(),
        })
    }

    fn peek_at(&self, ahead: usize) -> Tok {
        self.tokens.get(self.at + ahead).map_or(Tok::End, |t| t.tok)
    }

    fn bump(&mut self) -> Token {
        let token = self.peek();
        self.at += 1;
        token
    }

    fn pos(&self, token: Token) -> Pos {
        self.lines.pos(token.start)
    }

    fn lexeme(&self, token: Token) -> &'t str {
        self.text.get(token.start..token.end).unwrap_or("")
    }

    /// True if the next token is this keyword, unescaped.
    fn is_keyword(&self, word: &str) -> bool {
        self.keyword_at(0) == Some(word)
    }

    fn keyword_at(&self, ahead: usize) -> Option<&'t str> {
        let token = self.tokens.get(self.at + ahead).copied()?;
        match token.tok {
            Tok::Word { escaped: false } => {
                let text = self.lexeme(token);
                KEYWORDS.contains(&text).then_some(text)
            }
            _ => None,
        }
    }

    fn is_punct(&self, c: char) -> bool {
        self.peek().tok == Tok::Punct(c)
    }

    /// The refusal "`x` unexpected: y expected", `expected` being a fragment
    /// of the catalogue or, for a sign, `expect.literal`.
    fn unexpected(&self, expected: Message) -> Fail {
        let token = self.peek();
        let message = if token.tok == Tok::End {
            Message::new("E004.end").msg("expected", expected)
        } else {
            Message::new("E004")
                .text("found", self.lexeme(token))
                .msg("expected", expected)
        };
        (token.start, message)
    }

    fn literal(text: &str) -> Message {
        Message::new("expect.literal").text("text", text)
    }

    fn expect_punct(&mut self, c: char) -> Result<(), Fail> {
        if self.is_punct(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.unexpected(Self::literal(&c.to_string())))
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<(), Fail> {
        if self.is_keyword(word) {
            self.bump();
            Ok(())
        } else {
            Err(self.unexpected(Self::literal(word)))
        }
    }

    /// An `ID`: a word that is not a keyword, or a word escaped by `^`.
    fn ident(&mut self) -> Result<String, Fail> {
        let token = self.peek();
        match token.tok {
            Tok::Word { escaped } if escaped || self.keyword_at(0).is_none() => {
                self.bump();
                let text = self.lexeme(token);
                Ok(text.strip_prefix('^').unwrap_or(text).to_string())
            }
            _ => Err(self.unexpected(Message::new("expect.ident"))),
        }
    }

    fn is_ident(&self) -> bool {
        matches!(self.peek().tok, Tok::Word { escaped }
            if escaped || self.keyword_at(0).is_none())
    }

    fn qualified_id(&mut self) -> Result<QualifiedId, Fail> {
        let pos = self.pos(self.peek());
        let mut segments = vec![self.ident()?];
        while self.is_punct('.') {
            self.bump();
            segments.push(self.ident()?);
        }
        Ok(QualifiedId { segments, pos })
    }

    fn number(&mut self) -> Result<Number, Fail> {
        let token = self.peek();
        if token.tok != Tok::Number {
            return Err(self.unexpected(Message::new("expect.number")));
        }
        self.bump();
        Ok(Number {
            text: self.lexeme(token).to_string(),
            pos: self.pos(token),
        })
    }

    fn pair(&mut self) -> Result<(Number, Number), Fail> {
        let x = self.number()?;
        self.expect_punct(',')?;
        let y = self.number()?;
        Ok((x, y))
    }

    /// A string, without quotes, Java escapes resolved.
    fn string(&mut self) -> Result<String, Fail> {
        let token = self.peek();
        if token.tok != Tok::Str {
            return Err(self.unexpected(Message::new("expect.string")));
        }
        self.bump();
        let raw = self.lexeme(token);
        let inner = raw.get(1..raw.len().saturating_sub(1)).unwrap_or("");
        let mut out = String::new();
        let mut chars = inner.char_indices();
        while let Some((i, c)) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let at = token.start + 1 + i;
            let decoded = match chars.next().map(|(_, c)| c) {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('b') => '\u{8}',
                Some('f') => '\u{c}',
                Some(c @ ('\\' | '"' | '\'')) => c,
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                    let code = (hex.chars().count() == 4)
                        .then(|| u32::from_str_radix(&hex, 16).ok())
                        .flatten()
                        .and_then(char::from_u32);
                    match code {
                        Some(c) => c,
                        None => return Err((at, Message::new("E005"))),
                    }
                }
                _ => return Err((at, Message::new("E005"))),
            };
            out.push(decoded);
        }
        Ok(out)
    }

    // -- elements

    fn root(&mut self) -> Result<ElkNode, Fail> {
        let mut node = ElkNode {
            pos: self.pos(self.peek()),
            ..ElkNode::default()
        };
        if self.is_keyword("graph") {
            self.bump();
            node.id = Some(self.ident()?);
        }
        self.node_body(&mut node)?;
        if self.peek().tok != Tok::End {
            return Err(self.unexpected(Message::new("expect.element")));
        }
        Ok(node)
    }

    /// The content of a node, root or `node`, up to whatever is not part of it.
    fn node_body(&mut self, node: &mut ElkNode) -> Result<(), Fail> {
        node.layout = self.shape_layout()?;
        node.properties = self.properties()?;
        loop {
            match self.keyword_at(0) {
                Some("label") => node.labels.push(self.label()?),
                Some("port") => node.ports.push(self.port()?),
                Some("node") => node.children.push(self.node()?),
                Some("edge") => node.edges.push(self.edge()?),
                _ => return Ok(()),
            }
        }
    }

    fn node(&mut self) -> Result<ElkNode, Fail> {
        let pos = self.pos(self.peek());
        self.expect_keyword("node")?;
        let mut node = ElkNode {
            id: Some(self.ident()?),
            pos,
            ..ElkNode::default()
        };
        if self.is_punct('{') {
            self.bump();
            self.node_body(&mut node)?;
            self.expect_close(&["label", "port", "node", "edge"])?;
        }
        Ok(node)
    }

    fn port(&mut self) -> Result<ElkPort, Fail> {
        let pos = self.pos(self.peek());
        self.expect_keyword("port")?;
        let mut port = ElkPort {
            id: self.ident()?,
            pos,
            ..ElkPort::default()
        };
        if self.is_punct('{') {
            self.bump();
            port.layout = self.shape_layout()?;
            port.properties = self.properties()?;
            port.labels = self.labels()?;
            self.expect_close(&["label"])?;
        }
        Ok(port)
    }

    fn label(&mut self) -> Result<ElkLabel, Fail> {
        let pos = self.pos(self.peek());
        self.expect_keyword("label")?;
        let mut label = ElkLabel {
            pos,
            ..ElkLabel::default()
        };
        if self.is_ident() && self.peek_at(1) == Tok::Punct(':') {
            label.id = Some(self.ident()?);
            self.bump();
        }
        label.text = self.string()?;
        if self.is_punct('{') {
            self.bump();
            label.layout = self.shape_layout()?;
            label.properties = self.properties()?;
            label.labels = self.labels()?;
            self.expect_close(&["label"])?;
        }
        Ok(label)
    }

    fn labels(&mut self) -> Result<Vec<ElkLabel>, Fail> {
        let mut labels = Vec::new();
        while self.is_keyword("label") {
            labels.push(self.label()?);
        }
        Ok(labels)
    }

    fn edge(&mut self) -> Result<ElkEdge, Fail> {
        let pos = self.pos(self.peek());
        self.expect_keyword("edge")?;
        let mut edge = ElkEdge {
            pos,
            ..ElkEdge::default()
        };
        if self.is_ident() && self.peek_at(1) == Tok::Punct(':') {
            edge.id = Some(self.ident()?);
            self.bump();
        }
        edge.sources = self.qualified_ids()?;
        if self.peek().tok != Tok::Arrow {
            return Err(self.unexpected(Self::literal("->")));
        }
        self.bump();
        edge.targets = self.qualified_ids()?;
        if self.is_punct('{') {
            self.bump();
            edge.sections = self.edge_layout()?;
            edge.properties = self.properties()?;
            edge.labels = self.labels()?;
            self.expect_close(&["label"])?;
        }
        Ok(edge)
    }

    fn qualified_ids(&mut self) -> Result<Vec<QualifiedId>, Fail> {
        let mut ids = vec![self.qualified_id()?];
        while self.is_punct(',') {
            self.bump();
            ids.push(self.qualified_id()?);
        }
        Ok(ids)
    }

    /// The `}` that closes a block; otherwise, say what could have appeared there.
    fn expect_close(&mut self, elements: &[&str]) -> Result<(), Fail> {
        if self.is_punct('}') {
            self.bump();
            return Ok(());
        }
        let words: Vec<String> = elements.iter().map(|w| format!("`{w}`")).collect();
        let expected = Message::new("expect.close").text("elements", words.join(", "));
        Err(self.unexpected(expected))
    }

    // -- layout

    fn shape_layout(&mut self) -> Result<Shape, Fail> {
        let mut shape = Shape::default();
        if !self.is_keyword("layout") {
            return Ok(shape);
        }
        self.bump();
        self.expect_punct('[')?;
        self.group(&[Slot::Position, Slot::Size], |parser, slot| {
            match slot {
                Slot::Position => shape.position = Some(parser.pair()?),
                _ => shape.size = Some(parser.pair()?),
            }
            Ok(())
        })?;
        self.expect_punct(']')?;
        Ok(shape)
    }

    /// An `&` group: each of the keywords, at most once, in any order,
    /// each followed by `:` and by whatever `read` reads from it.
    fn group(
        &mut self,
        slots: &[Slot],
        mut read: impl FnMut(&mut Self, Slot) -> Result<(), Fail>,
    ) -> Result<(), Fail> {
        let mut seen: Vec<Slot> = Vec::new();
        loop {
            let Some(slot) = slots.iter().copied().find(|s| self.is_keyword(s.keyword())) else {
                return Ok(());
            };
            let token = self.bump();
            if seen.contains(&slot) {
                let message = Message::new("E006").text("key", slot.keyword());
                return Err((token.start, message));
            }
            seen.push(slot);
            self.expect_punct(':')?;
            read(self, slot)?;
        }
    }

    fn edge_layout(&mut self) -> Result<Vec<Section>, Fail> {
        if !self.is_keyword("layout") {
            return Ok(Vec::new());
        }
        self.bump();
        self.expect_punct('[')?;
        let mut sections = Vec::new();
        if self.is_keyword("section") {
            while self.is_keyword("section") {
                let token = self.bump();
                let pos = self.pos(token);
                let id = Some(self.ident()?);
                let mut outgoing_sections = Vec::new();
                if self.peek().tok == Tok::Arrow {
                    self.bump();
                    outgoing_sections.push(self.ident()?);
                    while self.is_punct(',') {
                        self.bump();
                        outgoing_sections.push(self.ident()?);
                    }
                }
                self.expect_punct('[')?;
                let mut section = self.section_body(pos)?;
                section.id = id;
                section.outgoing_sections = outgoing_sections;
                self.expect_punct(']')?;
                sections.push(section);
            }
        } else {
            let pos = self.pos(self.peek());
            sections.push(self.section_body(pos)?);
        }
        self.expect_punct(']')?;
        Ok(sections)
    }

    fn section_body(&mut self, pos: Pos) -> Result<Section, Fail> {
        let mut section = Section {
            pos,
            ..Section::default()
        };
        self.group(
            &[Slot::Incoming, Slot::Outgoing, Slot::Start, Slot::End],
            |parser, slot| {
                match slot {
                    Slot::Incoming => section.incoming = Some(parser.qualified_id()?),
                    Slot::Outgoing => section.outgoing = Some(parser.qualified_id()?),
                    Slot::Start => section.start = Some(parser.pair()?),
                    _ => section.end = Some(parser.pair()?),
                }
                Ok(())
            },
        )?;
        if self.is_keyword("bends") {
            self.bump();
            self.expect_punct(':')?;
            section.bends.push(self.pair()?);
            while self.is_punct('|') {
                self.bump();
                section.bends.push(self.pair()?);
            }
        }
        section.properties = self.properties()?;
        Ok(section)
    }

    // -- properties

    fn properties(&mut self) -> Result<Vec<Property>, Fail> {
        let mut properties = Vec::new();
        while self.is_ident() {
            properties.push(self.property()?);
        }
        Ok(properties)
    }

    fn property(&mut self) -> Result<Property, Fail> {
        let first = self.peek();
        let mut key = self.ident()?;
        // `PropertyKey` is written `hidden()`: no whitespace between its segments.
        let mut last_end = first.end;
        while self.is_punct('.') {
            let dot = self.peek();
            if dot.start != last_end
                || self
                    .tokens
                    .get(self.at + 1)
                    .is_none_or(|t| t.start != dot.end)
            {
                return Err(self.unexpected(Self::literal(":")));
            }
            self.bump();
            let segment = self.peek();
            key.push('.');
            key.push_str(&self.ident()?);
            last_end = segment.end;
        }
        self.expect_punct(':')?;
        let token = self.peek();
        let value = match token.tok {
            Tok::Str => Value::Str(self.string()?),
            Tok::Number => Value::Number(self.number()?),
            Tok::Word { .. } => match self.keyword_at(0) {
                Some("true") => {
                    self.bump();
                    Value::Bool(true)
                }
                Some("false") => {
                    self.bump();
                    Value::Bool(false)
                }
                Some("null") => {
                    self.bump();
                    Value::Null
                }
                Some(_) => return Err(self.unexpected(Message::new("expect.value"))),
                None => Value::Id(self.qualified_id()?.text()),
            },
            _ => return Err(self.unexpected(Message::new("expect.value"))),
        };
        Ok(Property {
            key,
            value,
            pos: self.pos(first),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> ElkNode {
        parse(text.as_bytes()).unwrap()
    }

    fn refusal(text: &str) -> (&'static str, u32, u32, String) {
        let d = parse(text.as_bytes()).unwrap_err();
        (d.code, d.pos.line, d.pos.col, d.text())
    }

    const SAMPLE: &str = "/* header */\n\
        graph g\n\
        layout [ size: 100, 50.5 ]\n\
        elk.direction: DOWN\n\
        node parent {\n\
        \x20 portConstraints: FIXED_SIDE // trailing\n\
        \x20 label \"Parent\\n\\u00e9\" { layout [size: 40, 10] }\n\
        \x20 port ^port { layout [position: -5, 2.25 size: 4, 4] side: WEST }\n\
        \x20 node child { layout [size: 10, 10] }\n\
        \x20 edge inner: child -> ^port { label 'c' }\n\
        }\n\
        node b\n\
        edge parent.^port -> b, parent.child {\n\
        \x20 layout [ incoming: parent.^port start: 1, 2 end: 3, 4 bends: 5, 6 | 7, 8 ]\n\
        \x20 edgeLabels.placement: HEAD\n\
        \x20 label \"e\" { layout [size: 1, 2] }\n\
        }\n";

    #[test]
    fn the_sample_is_read_whole() {
        let root = read(SAMPLE);
        assert_eq!(root.id.as_deref(), Some("g"));
        assert_eq!(root.pos, Pos { line: 2, col: 1 });
        let (w, h) = root.layout.size.clone().unwrap();
        assert_eq!((w.text.as_str(), h.text.as_str()), ("100", "50.5"));
        assert_eq!(w.pos, Pos { line: 3, col: 16 });
        assert_eq!(root.properties[0].key, "elk.direction");
        assert_eq!(root.properties[0].value, Value::Id("DOWN".into()));

        let parent = &root.children[0];
        assert_eq!(parent.id.as_deref(), Some("parent"));
        assert_eq!(parent.pos, Pos { line: 5, col: 1 });
        assert_eq!(parent.properties[0].value, Value::Id("FIXED_SIDE".into()));
        assert_eq!(parent.labels[0].text, "Parent\né");
        let port = &parent.ports[0];
        assert_eq!(port.id, "port");
        assert_eq!(port.properties[0].value, Value::Id("WEST".into()));
        let (x, y) = port.layout.position.clone().unwrap();
        assert_eq!((x.text.as_str(), y.text.as_str()), ("-5", "2.25"));
        assert_eq!(parent.children[0].id.as_deref(), Some("child"));
        let inner = &parent.edges[0];
        assert_eq!(inner.id.as_deref(), Some("inner"));
        assert_eq!(inner.sources[0].text(), "child");
        assert_eq!(inner.targets[0].text(), "port");
        assert_eq!(inner.labels[0].text, "c");

        assert_eq!(root.children[1].id.as_deref(), Some("b"));
        let edge = &root.edges[0];
        assert_eq!(edge.id, None);
        assert_eq!(edge.pos, Pos { line: 13, col: 1 });
        assert_eq!(edge.sources[0].segments, ["parent", "port"]);
        assert_eq!(edge.targets.len(), 2);
        assert_eq!(edge.targets[1].text(), "parent.child");
        let section = &edge.sections[0];
        assert_eq!(section.id, None);
        assert_eq!(section.incoming.as_ref().unwrap().text(), "parent.port");
        assert_eq!(section.start.as_ref().unwrap().0.text, "1");
        assert_eq!(section.end.as_ref().unwrap().1.text, "4");
        assert_eq!(section.bends.len(), 2);
        assert_eq!(section.bends[1].0.text, "7");
        assert_eq!(edge.properties[0].key, "edgeLabels.placement");
        assert_eq!(edge.labels[0].layout.size.as_ref().unwrap().1.text, "2");
    }

    #[test]
    fn an_empty_text_is_an_empty_root() {
        let empty = ElkNode {
            pos: Pos { line: 1, col: 1 },
            ..ElkNode::default()
        };
        assert_eq!(read(""), empty);
        // The root is placed on its first token, or at the end of the text.
        assert_eq!(read("\u{feff} // nothing\n").pos, Pos { line: 2, col: 1 });
    }

    #[test]
    fn named_sections_and_property_values_are_read() {
        let root = read(
            "edge a -> b {\n layout [\n  section s1 -> s2, s3 [ outgoing: b bends: 1, 1 x.y: 2 ]\n  \
             section s2 [ ]\n ]\n s: \"q\" i: a.b n: -1.5e3 t: true f: false z: null\n}",
        );
        let edge = &root.edges[0];
        let s1 = &edge.sections[0];
        assert_eq!(s1.id.as_deref(), Some("s1"));
        assert_eq!(s1.outgoing_sections, ["s2", "s3"]);
        assert_eq!(s1.outgoing.as_ref().unwrap().text(), "b");
        assert_eq!(s1.properties[0].key, "x.y");
        assert_eq!(edge.sections[1].id.as_deref(), Some("s2"));
        let values: Vec<&Value> = edge.properties.iter().map(|p| &p.value).collect();
        assert_eq!(
            values,
            [
                &Value::Str("q".into()),
                &Value::Id("a.b".into()),
                &Value::Number(Number {
                    text: "-1.5e3".into(),
                    pos: Pos { line: 6, col: 19 }
                }),
                &Value::Bool(true),
                &Value::Bool(false),
                &Value::Null,
            ]
        );
    }

    #[test]
    fn keywords_are_not_identifiers_unless_escaped() {
        assert_eq!(read("node ^node").children[0].id.as_deref(), Some("node"));
        assert_eq!(
            refusal("node node"),
            (
                "E004",
                1,
                6,
                "unexpected `node`: an identifier expected".into()
            )
        );
        // A keyword as a property value is refused too.
        assert_eq!(refusal("k: size").0, "E004");
    }

    #[test]
    fn a_number_is_kept_as_written() {
        let root = read("layout [size: +1, 2.50] k: 3e+2");
        let (w, h) = root.layout.size.unwrap();
        assert_eq!((w.text, h.text), ("+1".to_string(), "2.50".to_string()));
        assert_eq!(
            root.properties[0].value,
            Value::Number(Number {
                text: "3e+2".into(),
                pos: Pos { line: 1, col: 28 }
            })
        );
        // `1.` is not a float: the dot starts something else.
        assert_eq!(refusal("layout [size: 1., 2]").0, "E004");
    }

    #[test]
    fn text_that_is_not_utf8_is_refused_where_it_breaks() {
        let d = parse(b"node a\n n\xff").unwrap_err();
        assert_eq!((d.code, d.pos), ("E001", Pos { line: 2, col: 3 }));
    }

    #[test]
    fn unterminated_strings_and_comments_are_refused_at_their_start() {
        assert_eq!(
            refusal("node a\n label \"x"),
            ("E002", 2, 8, "unterminated string".into())
        );
        assert_eq!(
            refusal("node a /* x"),
            ("E002", 1, 8, "unterminated comment".into())
        );
        assert_eq!(refusal("label 'x\\'").0, "E002");
    }

    #[test]
    fn a_stray_character_is_refused() {
        assert_eq!(
            refusal("node a\n  é"),
            ("E003", 2, 3, "unexpected character `é`".into())
        );
        assert_eq!(refusal("node a ^1").0, "E003");
        assert_eq!(refusal("k: -").0, "E003");
        assert_eq!(refusal("k: +x").0, "E003");
        assert_eq!(refusal("k: ;").0, "E003");
    }

    #[test]
    fn what_was_expected_is_named() {
        let cases = [
            ("node", "unexpected end of text: an identifier expected"),
            (
                "node a {",
                "unexpected end of text: `}`, or `label`, `port`, `node`, `edge` expected",
            ),
            (
                "port p { node x }",
                "unexpected `node`: `}`, or `label` expected",
            ),
            ("edge a", "unexpected end of text: `->` expected"),
            ("edge a -> {", "unexpected `{`: an identifier expected"),
            ("label x", "unexpected `x`: a string expected"),
            ("layout [size: 1 2]", "unexpected `2`: `,` expected"),
            ("layout [size: a, 1]", "unexpected `a`: a number expected"),
            ("layout [bends: 1, 2]", "unexpected `bends`: `]` expected"),
            ("k:", "unexpected end of text: a value expected"),
            ("k: {", "unexpected `{`: a value expected"),
            (
                "}",
                "unexpected `}`: `label`, `port`, `node` or `edge` expected",
            ),
            (
                "node a k: 1",
                "unexpected `k`: `label`, `port`, `node` or `edge` expected",
            ),
            ("a .b: 1", "unexpected `.`: `:` expected"),
            ("a. b: 1", "unexpected `.`: `:` expected"),
            (
                "edge a -> b { layout [ section s [ ] start: 1, 2 ] }",
                "unexpected `start`: `]` expected",
            ),
        ];
        for (text, expected) in cases {
            let (code, _, _, message) = refusal(text);
            assert_eq!((code, message.as_str()), ("E004", expected), "{text}");
        }
        let (_, line, col, _) = refusal("node a\n  }");
        assert_eq!((line, col), (2, 3));
    }

    #[test]
    fn an_unknown_escape_is_refused_where_it_is() {
        assert_eq!(
            refusal("label \"a\\qb\""),
            ("E005", 1, 9, "unknown escape sequence in a string".into())
        );
        assert_eq!(refusal("label \"\\u12\"").0, "E005");
        assert_eq!(refusal("label \"\\uzzzz\"").0, "E005");
        assert_eq!(
            read("label \"\\\\\\\"\\t\\b\\f\\r\"").labels[0].text,
            "\\\"\t\u{8}\u{c}\r"
        );
    }

    #[test]
    fn a_layout_key_given_twice_is_refused() {
        assert_eq!(
            refusal("layout [size: 1, 1 position: 0, 0 size: 2, 2]"),
            (
                "E006",
                1,
                35,
                "`size` given twice in the same `layout`".into()
            )
        );
        assert_eq!(
            refusal("edge a -> b { layout [ end: 1, 1 end: 2, 2 ] }").0,
            "E006"
        );
        // In the other order, each once: accepted.
        let root = read("layout [position: 0, 0 size: 2, 2]");
        assert!(root.layout.position.is_some() && root.layout.size.is_some());
    }
}
