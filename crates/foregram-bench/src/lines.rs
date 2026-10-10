// ForegramPM — banc de mesure du placement : positions dans un texte source.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::Pos;

/// Début de chaque ligne d'un texte, pour passer d'un décalage en octets à
/// une ligne et une colonne en points de code. Sert aux lecteurs de formats
/// tiers, dont l'analyseur compte en octets ou relit le texte à chaque appel.
pub struct Lines<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Lines { text, starts }
    }

    pub fn pos(&self, offset: usize) -> Pos {
        let line = self.starts.partition_point(|&start| start <= offset);
        let start = self.starts.get(line.wrapping_sub(1)).copied().unwrap_or(0);
        let col = self
            .text
            .get(start..offset)
            .map_or(0, |s| s.chars().count());
        Pos {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            col: u32::try_from(col + 1).unwrap_or(u32::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts_turn_offsets_into_code_point_columns() {
        let lines = Lines::new("é\nab\n");
        assert_eq!(lines.pos(0), Pos { line: 1, col: 1 });
        assert_eq!(lines.pos(2), Pos { line: 1, col: 2 });
        assert_eq!(lines.pos(3), Pos { line: 2, col: 1 });
        assert_eq!(lines.pos(5), Pos { line: 2, col: 3 });
        assert_eq!(lines.pos(6), Pos { line: 3, col: 1 });
    }
}
