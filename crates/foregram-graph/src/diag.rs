// ForegramPM — diagnostics : position dans la source, code stable, message.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

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
        let line = if self.pos.is_none() {
            Message::new("diag.bare")
        } else {
            Message::new("diag.at")
                .text("line", self.pos.line)
                .text("col", self.pos.col)
        };
        line.text("code", self.code)
            .text("text", self.text_in(catalog))
            .render(catalog)
    }
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
