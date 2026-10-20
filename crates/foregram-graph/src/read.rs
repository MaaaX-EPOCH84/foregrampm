// ForegramPM — common tools for reading and validating the two formats.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::diag::{Diagnostic, Pos};
use crate::json::{Kind, Member, Value};
use crate::msg::Message;

/// Format version written by this version of the library. The versions
/// read run from 1 to this one: everything 2 adds is optional.
pub const FORMAT_VERSION: i64 = 2;

/// Bound on coordinates and dimensions, in hundredths of a pixel.
/// It lets the products of two differences fit in an `i64`.
pub const COORD_MAX: i64 = 1_000_000_000;

/// Bound on layer and rank numbers.
pub const LAYER_MAX: i64 = 1_000_000;

/// Maximum length of an identifier, in code points.
pub const IDENT_MAX_CHARS: usize = 255;

/// Names a key read in a format object, e.g. `width` (node).
pub(crate) fn field(key: &'static str, within: &'static str) -> Message {
    Message::new("field.of")
        .text("key", key)
        .msg("in", Message::new(within))
}

/// Names a key of an already identified element, e.g. node "A": `width`.
pub(crate) fn field_at(subject: Message, key: &'static str) -> Message {
    Message::new("field.at")
        .msg("subject", subject)
        .text("key", key)
}

/// A JSON object being read, which remembers the keys looked up so as to
/// refuse the others when it is closed.
pub(crate) struct Obj<'a> {
    pub pos: Pos,
    /// Key of the fragment that names the object: `in.node`.
    within: &'static str,
    members: &'a [Member],
    seen: Vec<bool>,
}

impl<'a> Obj<'a> {
    pub fn open(
        value: &'a Value,
        within: &'static str,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<Obj<'a>> {
        match &value.kind {
            Kind::Object(members) => Some(Obj {
                pos: value.pos,
                within,
                members,
                seen: vec![false; members.len()],
            }),
            _ => {
                let whole = Message::new(within);
                diags.push(type_error(value, whole, "type.object"));
                None
            }
        }
    }

    /// Value of an optional key.
    pub fn opt(&mut self, key: &str) -> Option<&'a Value> {
        let index = self.members.iter().position(|m| m.key == key)?;
        if let Some(seen) = self.seen.get_mut(index) {
            *seen = true;
        }
        self.members.get(index).map(|m| &m.value)
    }

    /// Value of a required key; its absence is a refusal.
    pub fn req(&mut self, key: &str, diags: &mut Vec<Diagnostic>) -> Option<&'a Value> {
        let value = self.opt(key);
        if value.is_none() {
            diags.push(Diagnostic::new(
                self.pos,
                Message::new("S003")
                    .text("key", key)
                    .msg("in", Message::new(self.within)),
            ));
        }
        value
    }

    /// Refuses every key that the reading did not look up.
    pub fn close(self, diags: &mut Vec<Diagnostic>) {
        for (member, seen) in self.members.iter().zip(&self.seen) {
            if !seen {
                diags.push(Diagnostic::new(
                    member.key_pos,
                    Message::new("S002")
                        .text("key", &member.key)
                        .msg("in", Message::new(self.within)),
                ));
            }
        }
    }
}

/// `expected` is the key of a type fragment: `type.integer`.
pub(crate) fn type_error(value: &Value, label: Message, expected: &'static str) -> Diagnostic {
    Diagnostic::new(
        value.pos,
        Message::new("S001")
            .msg("field", label)
            .msg("expected", Message::new(expected))
            .msg("found", Message::new(value.kind.name())),
    )
}

pub(crate) fn str_of<'a>(
    value: &'a Value,
    label: Message,
    diags: &mut Vec<Diagnostic>,
) -> Option<&'a str> {
    match &value.kind {
        Kind::Str(text) => Some(text),
        _ => {
            diags.push(type_error(value, label, "type.string"));
            None
        }
    }
}

pub(crate) fn int_of(value: &Value, label: Message, diags: &mut Vec<Diagnostic>) -> Option<i64> {
    match &value.kind {
        Kind::Int(number) => Some(*number),
        _ => {
            diags.push(type_error(value, label, "type.integer"));
            None
        }
    }
}

pub(crate) fn bool_of(value: &Value, label: Message, diags: &mut Vec<Diagnostic>) -> Option<bool> {
    match &value.kind {
        Kind::Bool(flag) => Some(*flag),
        _ => {
            diags.push(type_error(value, label, "type.boolean"));
            None
        }
    }
}

pub(crate) fn array_of<'a>(
    value: &'a Value,
    label: Message,
    diags: &mut Vec<Diagnostic>,
) -> Option<&'a [Value]> {
    match &value.kind {
        Kind::Array(items) => Some(items),
        _ => {
            diags.push(type_error(value, label, "type.array"));
            None
        }
    }
}

/// Reads a value taken from a closed list of words.
pub(crate) fn choice_of<T: Copy>(
    value: &Value,
    label: Message,
    table: &[(&'static str, T)],
    diags: &mut Vec<Diagnostic>,
) -> Option<T> {
    let text = str_of(value, label.clone(), diags)?;
    if let Some((_, choice)) = table.iter().find(|(word, _)| *word == text) {
        return Some(*choice);
    }
    let words: Vec<&str> = table.iter().map(|(word, _)| *word).collect();
    diags.push(Diagnostic::new(
        value.pos,
        Message::new("S006")
            .msg("field", label)
            .text("value", text)
            .text("words", words.join(", ")),
    ));
    None
}

/// Checks the two header keys common to the formats: `format` and `version`.
pub(crate) fn header(obj: &mut Obj<'_>, format: &str, diags: &mut Vec<Diagnostic>) {
    let root = |key| Message::new("field.root").text("key", key);
    if let Some(value) = obj.req("format", diags)
        && let Some(found) = str_of(value, root("format"), diags)
        && found != format
    {
        diags.push(Diagnostic::new(
            value.pos,
            Message::new("S004")
                .text("found", found)
                .text("expected", format),
        ));
    }
    if let Some(value) = obj.req("version", diags)
        && let Some(found) = int_of(value, root("version"), diags)
        && !(1..=FORMAT_VERSION).contains(&found)
    {
        diags.push(Diagnostic::new(
            value.pos,
            Message::new("S005")
                .text("found", found)
                .text("supported", FORMAT_VERSION),
        ));
    }
}

/// An identifier is a non-empty string, with no control character,
/// of at most 255 code points.
pub(crate) fn check_ident(text: &str, label: Message, pos: Pos, diags: &mut Vec<Diagnostic>) {
    let length = text.chars().count();
    let message = if text.is_empty() {
        Message::new("S007.empty")
    } else if text.chars().any(char::is_control) {
        Message::new("S007.control")
    } else if length > IDENT_MAX_CHARS {
        Message::new("S007.long")
            .text("length", length)
            .text("max", IDENT_MAX_CHARS)
    } else {
        return;
    };
    diags.push(Diagnostic::new(pos, message.msg("field", label)));
}

pub(crate) fn check_range(
    value: i64,
    min: i64,
    max: i64,
    label: Message,
    pos: Pos,
    diags: &mut Vec<Diagnostic>,
) {
    if value < min || value > max {
        diags.push(Diagnostic::new(
            pos,
            Message::new("S008")
                .msg("field", label)
                .text("value", value)
                .text("min", min)
                .text("max", max),
        ));
    }
}

/// Names an element in a message: by its identifier if it has a readable
/// one, by its rank otherwise. `kind` is `node`, `edge` or `port`.
pub(crate) fn item_name(kind: &str, id: &str, index: usize) -> Message {
    let readable = !(id.is_empty() || id.chars().any(char::is_control));
    match (kind, readable) {
        ("node", true) => Message::new("subject.node").text("id", id),
        ("node", false) => Message::new("subject.node-n").text("rank", index + 1),
        ("port", true) => Message::new("subject.port").text("id", id),
        ("port", false) => Message::new("subject.port-n").text("rank", index + 1),
        (_, true) => Message::new("subject.edge").text("id", id),
        (_, false) => Message::new("subject.edge-n").text("rank", index + 1),
    }
}
