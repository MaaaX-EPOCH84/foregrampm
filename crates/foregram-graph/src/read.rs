// ForegramPM — outils communs à la lecture et à la validation des deux formats.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::diag::{Diagnostic, Pos};
use crate::json::{Kind, Member, Value};

/// Version du format lue et écrite par cette version de la bibliothèque.
pub const FORMAT_VERSION: i64 = 1;

/// Borne des coordonnées et des dimensions, en centièmes de pixel.
/// Elle laisse les produits de deux écarts tenir dans un `i64`.
pub const COORD_MAX: i64 = 1_000_000_000;

/// Borne des numéros de couche et de rang.
pub const LAYER_MAX: i64 = 1_000_000;

/// Longueur maximale d'un identifiant, en points de code.
pub const IDENT_MAX_CHARS: usize = 255;

/// Un objet JSON en cours de lecture, qui retient les clés consultées
/// pour refuser les autres à la fermeture.
pub(crate) struct Obj<'a> {
    pub pos: Pos,
    what: &'static str,
    members: &'a [Member],
    seen: Vec<bool>,
}

impl<'a> Obj<'a> {
    pub fn open(
        value: &'a Value,
        what: &'static str,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<Obj<'a>> {
        match &value.kind {
            Kind::Object(members) => Some(Obj {
                pos: value.pos,
                what,
                members,
                seen: vec![false; members.len()],
            }),
            _ => {
                diags.push(type_error(value, what, "object"));
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
                "S003",
                self.pos,
                format!("required key `{key}` missing ({})", self.what),
            ));
        }
        value
    }

    /// Refuse toute clé que la lecture n'a pas consultée.
    pub fn close(self, diags: &mut Vec<Diagnostic>) {
        for (member, seen) in self.members.iter().zip(&self.seen) {
            if !seen {
                diags.push(Diagnostic::new(
                    "S002",
                    member.key_pos,
                    format!("unknown key `{}` ({})", member.key, self.what),
                ));
            }
        }
    }
}

pub(crate) fn type_error(value: &Value, label: &str, expected: &str) -> Diagnostic {
    Diagnostic::new(
        "S001",
        value.pos,
        format!("{label}: {expected} expected, got {}", value.kind.name()),
    )
}

pub(crate) fn str_of<'a>(
    value: &'a Value,
    label: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<&'a str> {
    match &value.kind {
        Kind::Str(text) => Some(text),
        _ => {
            diags.push(type_error(value, label, "string"));
            None
        }
    }
}

pub(crate) fn int_of(value: &Value, label: &str, diags: &mut Vec<Diagnostic>) -> Option<i64> {
    match &value.kind {
        Kind::Int(number) => Some(*number),
        _ => {
            diags.push(type_error(value, label, "integer"));
            None
        }
    }
}

pub(crate) fn array_of<'a>(
    value: &'a Value,
    label: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<&'a [Value]> {
    match &value.kind {
        Kind::Array(items) => Some(items),
        _ => {
            diags.push(type_error(value, label, "array"));
            None
        }
    }
}

/// Lit une valeur prise dans une liste fermée de mots.
pub(crate) fn choice_of<T: Copy>(
    value: &Value,
    label: &str,
    table: &[(&'static str, T)],
    diags: &mut Vec<Diagnostic>,
) -> Option<T> {
    let text = str_of(value, label, diags)?;
    if let Some((_, choice)) = table.iter().find(|(word, _)| *word == text) {
        return Some(*choice);
    }
    let words: Vec<&str> = table.iter().map(|(word, _)| *word).collect();
    diags.push(Diagnostic::new(
        "S006",
        value.pos,
        format!(
            "{label}: unknown value `{text}`; expected one of {}",
            words.join(", ")
        ),
    ));
    None
}

/// Vérifie les deux clés d'en-tête communes aux formats : `format` et `version`.
pub(crate) fn header(obj: &mut Obj<'_>, format: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(value) = obj.req("format", diags)
        && let Some(found) = str_of(value, "`format`", diags)
        && found != format
    {
        diags.push(Diagnostic::new(
            "S004",
            value.pos,
            format!("unexpected format `{found}`: `{format}` expected"),
        ));
    }
    if let Some(value) = obj.req("version", diags)
        && let Some(found) = int_of(value, "`version`", diags)
        && found != FORMAT_VERSION
    {
        diags.push(Diagnostic::new(
            "S005",
            value.pos,
            format!("version {found} not supported: only version {FORMAT_VERSION} is read"),
        ));
    }
}

/// Un identifiant est une chaîne non vide, sans caractère de contrôle,
/// de 255 points de code au plus.
pub(crate) fn check_ident(text: &str, label: &str, pos: Pos, diags: &mut Vec<Diagnostic>) {
    let problem = if text.is_empty() {
        Some("empty".to_string())
    } else if text.chars().any(char::is_control) {
        Some("contains a control character".to_string())
    } else {
        let length = text.chars().count();
        (length > IDENT_MAX_CHARS)
            .then(|| format!("too long ({length} characters, at most {IDENT_MAX_CHARS})"))
    };
    if let Some(problem) = problem {
        diags.push(Diagnostic::new(
            "S007",
            pos,
            format!("{label}: identifier {problem}"),
        ));
    }
}

pub(crate) fn check_range(
    value: i64,
    min: i64,
    max: i64,
    label: &str,
    pos: Pos,
    diags: &mut Vec<Diagnostic>,
) {
    if value < min || value > max {
        diags.push(Diagnostic::new(
            "S008",
            pos,
            format!("{label}: {value} outside the range {min} to {max}"),
        ));
    }
}

/// Désigne un élément dans un message : par son identifiant s'il en a un
/// de lisible, par son rang sinon.
pub(crate) fn item_name(kind: &str, id: &str, index: usize) -> String {
    if id.is_empty() || id.chars().any(char::is_control) {
        format!("{kind} #{}", index + 1)
    } else {
        format!("{kind} \"{id}\"")
    }
}
