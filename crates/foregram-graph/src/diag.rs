// ForegramPM — diagnostics : position dans la source, code stable, message.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

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
/// du message ; c'est lui qu'un programme doit lire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub pos: Pos,
    pub message: String,
}

impl Diagnostic {
    pub fn new(code: &'static str, pos: Pos, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            pos,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pos.is_none() {
            write!(f, "error[{}]: {}", self.code, self.message)
        } else {
            write!(
                f,
                "{}:{}: error[{}]: {}",
                self.pos.line, self.pos.col, self.code, self.message
            )
        }
    }
}

/// Ordre de restitution : par position, puis par code. Le tri est stable :
/// à position et code égaux, l'ordre de relevé est gardé, et il ne dépend
/// que du contenu. Deux exécutions rendent donc la même liste.
pub(crate) fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| (a.pos, a.code).cmp(&(b.pos, b.code)));
}

/// « (ligne N) » quand la position est connue, rien sinon.
pub(crate) fn at_line(pos: Pos) -> String {
    if pos.is_none() {
        String::new()
    } else {
        format!(" (line {})", pos.line)
    }
}
