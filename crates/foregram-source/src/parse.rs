// ForegramPM — parser of the source format: recursive descent over the tokens.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use foregram_graph::{Diagnostic, Message, Pos};

use crate::lex::{self, Tok, Token};
use crate::model::{Comment, Content, Directive, Document, Element, Item, Value};

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
        let document = parser.document();
        if parser.diags.is_empty() {
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
    /// A `}`, a `;`, a comment, a line end or the end of the file, not
    /// consumed.
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
                Tok::Close | Tok::Semi | Tok::Eol | Tok::Comment(_) => break,
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

    /// The comment that ends the line of the item just read, after at most
    /// one `;`: consumed and returned. Any other comment stands on its own
    /// and goes to the item that follows it.
    fn trailing(&mut self) -> Option<Comment> {
        let mut j = self.i;
        if matches!(self.toks.get(j), Some(t) if t.tok == Tok::Semi) {
            j += 1;
        }
        let token = self.toks.get(j)?;
        let Tok::Comment(text) = &token.tok else {
            return None;
        };
        self.i = j + 1;
        Some(Comment {
            text: text.clone(),
            pos: token.pos,
        })
    }

    /// A comment on its own line, consumed and kept for the item that
    /// follows.
    fn standalone(&mut self, pending: &mut Vec<Comment>) {
        if let Some(token) = self.peek()
            && let Tok::Comment(text) = &token.tok
        {
            pending.push(Comment {
                text: text.clone(),
                pos: token.pos,
            });
            self.i += 1;
        }
    }

    fn document(&mut self) -> Document {
        let mut document = Document::default();
        let mut keys = BTreeMap::new();
        let mut started = false;
        // The comments on their own line not yet attached to an item.
        let mut pending: Vec<Comment> = Vec::new();
        // Whether no item has been kept yet: comments before the first one are
        // the preamble.
        let mut no_item_yet = true;
        // A run of lines that are not element heads is refused once.
        let mut stray = false;
        while let Some(token) = self.peek() {
            match token.tok {
                Tok::Eol | Tok::Semi => {
                    self.i += 1;
                    continue;
                }
                Tok::Comment(_) => {
                    self.standalone(&mut pending);
                    continue;
                }
                Tok::Close => {
                    let pos = token.pos;
                    self.i += 1;
                    self.refuse(pos, Message::new("SRC006"));
                    continue;
                }
                _ => {}
            }
            let (values, stop) = self.values();
            if let Stop::Open(open) = stop {
                stray = false;
                started = true;
                if let Some(mut element) = self.element(&values, open, 1) {
                    if no_item_yet {
                        document.preamble = std::mem::take(&mut pending);
                        no_item_yet = false;
                    }
                    element.before = std::mem::take(&mut pending);
                    document.elements.push(element);
                }
            } else if is_directive(&values) {
                stray = false;
                if !started {
                    if let Some(mut directive) = self.directive(values, &mut keys) {
                        if no_item_yet {
                            document.preamble = std::mem::take(&mut pending);
                            no_item_yet = false;
                        }
                        directive.before = std::mem::take(&mut pending);
                        directive.after = self.trailing();
                        document.directives.push(directive);
                    }
                } else if let Some(first) = values.first() {
                    let key = first.text.get(1..).unwrap_or("");
                    let message = if key.is_empty() {
                        Message::new("SRC011")
                    } else {
                        Message::new("SRC010").text("key", key)
                    };
                    self.refuse(first.pos, message);
                }
            } else if let Some(first) = values.first()
                && !stray
            {
                stray = true;
                self.refuse(first.pos, Message::new("SRC008.top"));
            }
        }
        if no_item_yet {
            document.preamble = pending;
        } else {
            document.end = pending;
        }
        document
    }

    /// A directive or an attribute, refused when its key is missing or was
    /// already given in the same header or block. Its comments are attached
    /// by the caller.
    fn directive(
        &mut self,
        values: Vec<Value>,
        keys: &mut BTreeMap<String, Pos>,
    ) -> Option<Directive> {
        let mut values = values.into_iter();
        let at = values.next()?;
        let key = at.text.get(1..).unwrap_or("").to_string();
        if key.is_empty() {
            self.refuse(at.pos, Message::new("SRC011"));
            return None;
        }
        if let Some(first) = keys.get(&key) {
            let message = Message::new("SRC013")
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
            before: Vec::new(),
            after: None,
        })
    }

    /// The head before a `{`: kind, identifier, optional label. Refusals are
    /// reported here; `None` when the head cannot make an element.
    fn head(&mut self, head: &[Value], open: Pos) -> Option<Head> {
        let Some(kind) = head.first() else {
            self.refuse(open, Message::new("SRC008.kind"));
            return None;
        };
        if kind.quoted || kind.text.starts_with('@') {
            self.refuse(kind.pos, Message::new("SRC008.kind"));
            return None;
        }
        let Some(id) = head.get(1) else {
            self.refuse(open, Message::new("SRC008.id"));
            return None;
        };
        if id.quoted {
            self.refuse(id.pos, Message::new("SRC008.id"));
            return None;
        }
        let label = match head.get(2) {
            None => None,
            Some(label) if label.quoted => Some(label.clone()),
            Some(other) => {
                self.refuse(other.pos, Message::new("SRC008.label"));
                return None;
            }
        };
        if let Some(extra) = head.get(3) {
            self.refuse(extra.pos, Message::new("SRC008.label"));
            return None;
        }
        if !is_identifier(&id.text) {
            self.refuse(id.pos, Message::new("SRC009").text("id", &id.text));
            return None;
        }
        if let Some(first) = self.ids.get(&id.text) {
            let message = Message::new("SRC012")
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
    /// refusals. The comments before the head are attached by the caller.
    fn element(&mut self, head: &[Value], open: Pos, depth: usize) -> Option<Element> {
        let header = self.head(head, open);
        self.i += 1;
        if depth > NESTING_MAX {
            self.refuse(open, Message::new("SRC014").text("max", NESTING_MAX));
            self.skip_block(open);
            return None;
        }
        let after_open = self.trailing();
        let mut items = Vec::new();
        let mut keys = BTreeMap::new();
        let mut pending: Vec<Comment> = Vec::new();
        let close = loop {
            let Some(token) = self.peek() else {
                self.refuse(open, Message::new("SRC007"));
                return None;
            };
            match token.tok {
                Tok::Eol | Tok::Semi => {
                    self.i += 1;
                    continue;
                }
                Tok::Comment(_) => {
                    self.standalone(&mut pending);
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
                if let Some(mut child) = self.element(&values, inner, depth + 1) {
                    child.before = std::mem::take(&mut pending);
                    items.push(Item::Element(child));
                }
            } else if is_directive(&values) {
                if let Some(mut attribute) = self.directive(values, &mut keys) {
                    attribute.before = std::mem::take(&mut pending);
                    attribute.after = self.trailing();
                    items.push(Item::Attribute(attribute));
                }
            } else if let Some(first) = values.first() {
                let pos = first.pos;
                let before = std::mem::take(&mut pending);
                let after = self.trailing();
                items.push(Item::Content(Content {
                    values,
                    pos,
                    before,
                    after,
                }));
            }
        };
        let after_close = self.trailing();
        let head = header?;
        Some(Element {
            kind: head.kind,
            id: head.id,
            label: head.label,
            items,
            pos: head.pos,
            id_pos: head.id_pos,
            close,
            before: Vec::new(),
            after_open,
            after_close,
            end: pending,
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
        self.refuse(open, Message::new("SRC007"));
    }
}
