// ForegramPM — diagnostics : position dans la source, code stable, message.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

use crate::json::quote;
use crate::msg::{Catalog, Message};

/// Position dans un texte source, comptée à partir de 1.
///
/// La colonne compte des points de code Unicode, pas des octets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl Pos {
    /// Absence de position : l'élément n'a pas été lu dans un texte.
    pub const NONE: Pos = Pos { line: 0, col: 0 };

    pub fn is_none(self) -> bool {
        self.line == 0
    }
}

/// Un refus : où, lequel, pourquoi.
///
/// Le code est stable d'une version à l'autre et ne dépend pas de la langue
/// du message ; c'est lui qu'un programme doit lire. Le message ne porte pas
/// sa phrase : elle est composée à l'affichage, dans la langue choisie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub pos: Pos,
    pub message: Message,
}

impl Diagnostic {
    /// Le code est la clé du message, sans sa variante : `J005` pour `J005.zero`.
    pub fn new(pos: Pos, message: Message) -> Self {
        let code = message.key.split('.').next().unwrap_or(message.key);
        // Chaque refus produit par les tests est confronté au catalogue anglais :
        // une clé absente ou un argument oublié s'y voit tout de suite.
        debug_assert_eq!(message.check(Catalog::english()), Ok(()));
        Diagnostic { code, pos, message }
    }

    /// Le message seul, composé dans la langue du catalogue.
    pub fn text_in(&self, catalog: &Catalog) -> String {
        self.message.render(catalog)
    }

    /// Le message seul, en anglais.
    pub fn text(&self) -> String {
        self.text_in(Catalog::english())
    }

    /// Le refus entier — position, code, message — dans la langue du catalogue.
    pub fn render(&self, catalog: &Catalog) -> String {
        self.line().render(catalog)
    }

    /// Le refus précédé du fichier où il a été lu, comme une ligne de
    /// commande l'écrit : `g.json:7:5: error[G001]: …`, ou `g.json: error[…]`
    /// sans position.
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

    /// Le refus en un objet JSON sur une ligne, pour un programme : le
    /// fichier, le code, la position (`null` sans position), la clé et les
    /// arguments du message, puis sa phrase dans la langue du catalogue.
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

/// Les refus d'un fichier tels qu'une ligne de commande les écrit : un par
/// ligne, chacun précédé du fichier, puis une ligne qui les compte.
pub fn file_report(file: &str, diags: &[Diagnostic], catalog: &Catalog) -> Vec<String> {
    let mut lines: Vec<String> = diags
        .iter()
        .map(|diag| diag.render_in_file(file, catalog))
        .collect();
    let count = Message::count(diags.len(), "count.error.one", "count.error.other");
    lines.push(
        Message::new("diag.refused")
            .text("file", file)
            .msg("count", count)
            .render(catalog),
    );
    lines
}

impl fmt::Display for Diagnostic {
    /// Le refus entier, en anglais.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(Catalog::english()))
    }
}

/// Ordre de restitution : par position, puis par code. Le tri est stable :
/// à position et code égaux, l'ordre de relevé est gardé, et il ne dépend
/// que du contenu. Deux exécutions rendent donc la même liste.
pub(crate) fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
}
