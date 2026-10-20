// ForegramPM — diagnostics: source position, stable code, message.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

use crate::json::quote;
use crate::msg::{Catalog, Message};

/// Position in a source text, counted from 1.
///
/// The column counts Unicode code points, not bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    /// Line number, counted from 1; 0 means no position.
    pub line: u32,
    /// Column number in code points, counted from 1.
    pub col: u32,
}

impl Pos {
    /// No position: the element was not read from a text.
    pub const NONE: Pos = Pos { line: 0, col: 0 };

    /// Whether this position is [`Pos::NONE`] (line 0).
    pub fn is_none(self) -> bool {
        self.line == 0
    }
}

/// A refusal: where, which, why.
///
/// The code is stable from one version to the next and does not depend on the
/// message language; a program should read it. The message carries no
/// sentence: it is composed at display time, in the chosen language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code of the refusal: the message key without its variant.
    pub code: &'static str,
    /// Where the refusal was found; [`Pos::NONE`] if there is no position.
    pub pos: Pos,
    /// The message: a key and arguments, with no sentence of its own.
    pub message: Message,
}

impl Diagnostic {
    /// The code is the message key minus its variant: `J005` for `J005.zero`.
    pub fn new(pos: Pos, message: Message) -> Self {
        let code = message.key.split('.').next().unwrap_or(message.key);
        // Every refusal produced by the tests is checked against the English
        // catalogue: a missing key or a forgotten argument shows up at once.
        debug_assert_eq!(message.check(Catalog::english()), Ok(()));
        Diagnostic { code, pos, message }
    }

    /// The message alone, composed in the catalogue's language.
    pub fn text_in(&self, catalog: &Catalog) -> String {
        self.message.render(catalog)
    }

    /// The message alone, in English.
    pub fn text(&self) -> String {
        self.text_in(Catalog::english())
    }

    /// The whole refusal (position, code, message) in the catalogue's language.
    pub fn render(&self, catalog: &Catalog) -> String {
        self.line().render(catalog)
    }

    /// The refusal preceded by the file it was read from, as a command line
    /// writes it: `g.json:7:5: error[G001]: …`, or `g.json: error[…]`
    /// without a position.
    pub fn render_in_file(&self, file: &str, catalog: &Catalog) -> String {
        let key = if self.pos.is_none() {
            "diag.file-bare"
        } else {
            "diag.file-at"
        };
        Message::new(key)
            .text("file", file)
            .msg("diag", self.line())
            .render(catalog)
    }

    /// The refusal as a one-line JSON object, for a program: the file, the
    /// code, the position (`null` if none), the message key and arguments,
    /// then its sentence in the catalogue's language.
    pub fn to_json(&self, file: &str, catalog: &Catalog) -> String {
        let (line, col) = if self.pos.is_none() {
            ("null".to_string(), "null".to_string())
        } else {
            (self.pos.line.to_string(), self.pos.col.to_string())
        };
        format!(
            "{{\"kind\": \"diagnostic\", \"file\": {}, \"code\": {}, \"line\": {line}, \"col\": {col}, \"key\": {}, \"args\": {}, \"text\": {}}}",
            quote(file),
            quote(self.code),
            quote(self.message.key),
            self.message.args_json(),
            quote(&self.text_in(catalog))
        )
    }

    fn line(&self) -> Message {
        let line = if self.pos.is_none() {
            Message::new("diag.bare")
        } else {
            Message::new("diag.at")
                .text("line", self.pos.line)
                .text("col", self.pos.col)
        };
        line.text("code", self.code)
            .msg("text", self.message.clone())
    }
}

/// The refusals of a file as a command line writes them: one per line, each
/// preceded by the file, then a line that counts them.
pub fn file_report(file: &str, diags: &[Diagnostic], catalog: &Catalog) -> Vec<String> {
    let mut lines: Vec<String> = diags
        .iter()
        .map(|diag| diag.render_in_file(file, catalog))
        .collect();
    let count = Message::count(diags.len(), "count.error");
    lines.push(
        Message::new("diag.refused")
            .text("file", file)
            .msg("count", count)
            .render(catalog),
    );
    lines
}

impl fmt::Display for Diagnostic {
    /// The whole refusal, in English.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(Catalog::english()))
    }
}

/// Reporting order: by position, then by code. The sort is stable: with equal
/// position and code, the order of discovery is kept, and it depends only on
/// the content. Two runs therefore return the same list.
pub(crate) fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
}
