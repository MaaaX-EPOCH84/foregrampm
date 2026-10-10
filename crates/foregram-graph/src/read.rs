// ForegramPM — outils communs à la lecture et à la validation des deux formats.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::diag::{Diagnostic, Pos};
use crate::json::{Kind, Member, Value};
use crate::msg::Message;

/// Version du format écrite par cette version de la bibliothèque. Les
/// versions lues vont de 1 à celle-ci : tout ce que la 2 ajoute est facultatif.
pub const FORMAT_VERSION: i64 = 2;

/// Borne des coordonnées et des dimensions, en centièmes de pixel.
/// Elle laisse les produits de deux écarts tenir dans un `i64`.
pub const COORD_MAX: i64 = 1_000_000_000;

/// Borne des numéros de couche et de rang.
pub const LAYER_MAX: i64 = 1_000_000;

/// Longueur maximale d'un identifiant, en points de code.
pub const IDENT_MAX_CHARS: usize = 255;

/// Désigne une clé lue dans un objet du format : « `width` (node) ».
pub(crate) fn field(key: &'static str, within: &'static str) -> Message {
    Message::new("field.of")
        .text("key", key)
        .msg("in", Message::new(within))
}

/// Désigne une clé d'un élément déjà identifié : « node "A": `width` ».
pub(crate) fn field_at(subject: Message, key: &'static str) -> Message {
    Message::new("field.at")
        .msg("subject", subject)
        .text("key", key)
}

/// Un objet JSON en cours de lecture, qui retient les clés consultées
/// pour refuser les autres à la fermeture.
pub(crate) struct Obj<'a> {
    pub pos: Pos,
    /// Clé du fragment qui nomme l'objet : `in.node`.
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

    /// Valeur d'une clé facultative.
    pub fn opt(&mut self, key: &str) -> Option<&'a Value> {
        let index = self.members.iter().position(|m| m.key == key)?;
        if let Some(seen) = self.seen.get_mut(index) {
            *seen = true;
        }
        self.members.get(index).map(|m| &m.value)
    }

    /// Valeur d'une clé obligatoire ; son absence est un refus.
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

    /// Refuse toute clé que la lecture n'a pas consultée.
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

/// `expected` est la clé d'un fragment de type : `type.integer`.
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

/// Lit une valeur prise dans une liste fermée de mots.
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

/// Vérifie les deux clés d'en-tête communes aux formats : `format` et `version`.
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

/// Un identifiant est une chaîne non vide, sans caractère de contrôle,
/// de 255 points de code au plus.
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

/// Désigne un élément dans un message : par son identifiant s'il en a un
/// de lisible, par son rang sinon. `kind` vaut `node`, `edge` ou `port`.
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
