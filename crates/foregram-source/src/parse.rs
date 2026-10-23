// ForegramPM — parser of the source format: recursive descent over the tokens.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use foregram_graph::{Diagnostic, Message, Pos};

use crate::lex::{self, Tok, Token};
use crate::model::{Content, Directive, Document, Element, Item, Value};

/// The deepest nesting of blocks the format accepts.
pub const NESTING_MAX: usize = 32;

impl Document {
    /// Reads a source file.
    ///
    /// Returns the document, or every refusal found in the file, sorted by
    /// position — never a partial document. Bytes that are not UTF-8 stop the
    /// reading at once; any other refusal lets the reader go on after the
    /// faulty item, so one pass reports what it can.
    pub fn parse(bytes: &[u8]) -> Result<Document, Vec<Diagnostic>> {
        let text = lex::decode(bytes).map_err(|diag| vec![diag])?;
        let lexed = lex::lex(text);
        let mut parser = Parser {
            toks: &lexed.tokens,
            i: 0,
            diags: lexed.diags,
            ids: BTreeMap::new(),
        };
        let mut document = parser.document();
        if parser.diags.is_empty() {
            document.comments = lexed.comments;
            Ok(document)
        } else {
            let mut diags = parser.diags;
            diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
            Err(diags)
        }
    }
}

/// What ended a run of values.
enum Stop {
    /// A `{`, not consumed: the values are an element head.
    Open(Pos),
    /// A `}`, a `;`, a line end or the end of the file, not consumed.
    End,
}

struct Parser<'a> {
    toks: &'a [Token],
    i: usize,
    diags: Vec<Diagnostic>,
    /// Every identifier defined so far, with its position.
    ids: BTreeMap<String, Pos>,
}

/// An identifier: ASCII letters, digits and `_`, not starting with a digit.
fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether a run of values is a directive or an attribute: it starts with
/// a word that starts with `@`.
fn is_directive(values: &[Value]) -> bool {
    values
        .first()
        .is_some_and(|v| !v.quoted && v.text.starts_with('@'))
}

/// A valid element head.
struct Head {
    kind: String,
    id: String,
    id_pos: Pos,
    label: Option<Value>,
    pos: Pos,
}

impl<'a> Parser<'a> {
    fn refuse(&mut self, pos: Pos, message: Message) {
        self.diags.push(Diagnostic::new(pos, message));
    }

    fn peek(&self) -> Option<&'a Token> {
        self.toks.get(self.i)
    }

    /// The values from here up to the end of the item, which is not
    /// consumed.
    fn values(&mut self) -> (Vec<Value>, Stop) {
        let mut values = Vec::new();
        while let Some(token) = self.peek() {
            let (text, quoted) = match &token.tok {
                Tok::Word(text) => (text, false),
                Tok::Str(text) => (text, true),
                Tok::Open => return (values, Stop::Open(token.pos)),
                Tok::Close | Tok::Semi | Tok::Eol => break,
            };
            values.push(Value {
                text: text.clone(),
                quoted,
                pos: token.pos,
            });
            self.i += 1;
        }
        (values, Stop::End)
    }

    fn document(&mut self) -> Document {
        let mut document = Document::default();
        let mut keys = BTreeMap::new();
        let mut started = false;
        // A run of lines that are not element heads is refused once.
        let mut stray = false;
        while let Some(token) = self.peek() {
            match token.tok {
                Tok::Eol | Tok::Semi => {
                    self.i += 1;
                    continue;
                }
                Tok::Close => {
                    let pos = token.pos;
                    self.i += 1;
                    self.refuse(pos, Message::new("T006"));
                    continue;
                }
                _ => {}
            }
            let (values, stop) = self.values();
            if let Stop::Open(open) = stop {
                stray = false;
                started = true;
                if let Some(element) = self.element(&values, open, 1) {
                    document.elements.push(element);
                }
            } else if is_directive(&values) {
                stray = false;
                if !started {
                    if let Some(directive) = self.directive(values, &mut keys) {
                        document.directives.push(directive);
                    }
                } else if let Some(first) = values.first() {
                    let key = first.text.get(1..).unwrap_or("");
                    let message = if key.is_empty() {
                        Message::new("T011")
                    } else {
                        Message::new("T010").text("key", key)
                    };
                    self.refuse(first.pos, message);
                }
            } else if let Some(first) = values.first()
                && !stray
            {
                stray = true;
                self.refuse(first.pos, Message::new("T008.top"));
            }
        }
        document
    }

    /// A directive or an attribute, refused when its key is missing or was
    /// already given in the same header or block.
    fn directive(
        &mut self,
        values: Vec<Value>,
        keys: &mut BTreeMap<String, Pos>,
    ) -> Option<Directive> {
        let mut values = values.into_iter();
        let at = values.next()?;
        let key = at.text.get(1..).unwrap_or("").to_string();
        if key.is_empty() {
            self.refuse(at.pos, Message::new("T011"));
            return None;
        }
        if let Some(first) = keys.get(&key) {
            let message = Message::new("T013")
                .text("key", &key)
                .text("line", first.line)
                .text("col", first.col);
            self.refuse(at.pos, message);
            return None;
        }
        keys.insert(key.clone(), at.pos);
        Some(Directive {
            key,
            values: values.collect(),
            pos: at.pos,
        })
    }

    /// The head before a `{`: kind, identifier, optional label. Refusals are
    /// reported here; `None` when the head cannot make an element.
    fn head(&mut self, head: &[Value], open: Pos) -> Option<Head> {
        let Some(kind) = head.first() else {
            self.refuse(open, Message::new("T008.kind"));
            return None;
        };
        if kind.quoted || kind.text.starts_with('@') {
            self.refuse(kind.pos, Message::new("T008.kind"));
            return None;
        }
        let Some(id) = head.get(1) else {
            self.refuse(open, Message::new("T008.id"));
            return None;
        };
        if id.quoted {
            self.refuse(id.pos, Message::new("T008.id"));
            return None;
        }
        let label = match head.get(2) {
            None => None,
            Some(label) if label.quoted => Some(label.clone()),
            Some(other) => {
                self.refuse(other.pos, Message::new("T008.label"));
                return None;
            }
        };
        if let Some(extra) = head.get(3) {
            self.refuse(extra.pos, Message::new("T008.label"));
            return None;
        }
        if !is_identifier(&id.text) {
            self.refuse(id.pos, Message::new("T009").text("id", &id.text));
            return None;
        }
        if let Some(first) = self.ids.get(&id.text) {
            let message = Message::new("T012")
                .text("id", &id.text)
                .text("line", first.line)
                .text("col", first.col);
            self.refuse(id.pos, message);
            return None;
        }
        self.ids.insert(id.text.clone(), id.pos);
        Some(Head {
            kind: kind.text.clone(),
            id: id.text.clone(),
            id_pos: id.pos,
            label,
            pos: kind.pos,
        })
    }

    /// An element, from its head to its `}`; `depth` is the level of the
    /// block it opens, 1 at the top. A faulty head still opens its block, so
    /// that the braces stay paired and the block is read for its own
    /// refusals.
    fn element(&mut self, head: &[Value], open: Pos, depth: usize) -> Option<Element> {
        let header = self.head(head, open);
        self.i += 1;
        if depth > NESTING_MAX {
            self.refuse(open, Message::new("T014").text("max", NESTING_MAX));
            self.skip_block(open);
            return None;
        }
        let mut items = Vec::new();
        let mut keys = BTreeMap::new();
        let close = loop {
            let Some(token) = self.peek() else {
                self.refuse(open, Message::new("T007"));
                return None;
            };
            match token.tok {
                Tok::Eol | Tok::Semi => {
                    self.i += 1;
                    continue;
                }
                Tok::Close => {
                    let pos = token.pos;
                    self.i += 1;
                    break pos;
                }
                _ => {}
            }
            let (values, stop) = self.values();
            if let Stop::Open(inner) = stop {
                if let Some(child) = self.element(&values, inner, depth + 1) {
                    items.push(Item::Element(child));
                }
            } else if is_directive(&values) {
                if let Some(attribute) = self.directive(values, &mut keys) {
                    items.push(Item::Attribute(attribute));
                }
            } else if let Some(first) = values.first() {
                let pos = first.pos;
                items.push(Item::Content(Content { values, pos }));
            }
        };
        let head = header?;
        Some(Element {
            kind: head.kind,
            id: head.id,
            label: head.label,
            items,
            pos: head.pos,
            id_pos: head.id_pos,
            close,
        })
    }

    /// Skips a block too deep to be read, without descending: its braces
    /// are only counted.
    fn skip_block(&mut self, open: Pos) {
        let mut depth = 1usize;
        while let Some(token) = self.peek() {
            self.i += 1;
            match token.tok {
                Tok::Open => depth += 1,
                Tok::Close => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
        self.refuse(open, Message::new("T007"));
    }
}
