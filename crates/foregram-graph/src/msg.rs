// ForegramPM — messages localisables : une clé, des arguments, et un catalogue par langue.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Un message ne porte pas sa phrase : il porte une clé et des arguments, et
//! la phrase est composée à l'affichage depuis le catalogue de la langue
//! choisie. Un argument est soit un texte rendu tel quel (un identifiant, une
//! valeur), soit un autre message, traduit à son tour (« node "A" »,
//! « `width` (node) »).
//!
//! Un catalogue est un fichier texte, une entrée par ligne :
//!
//! ```text
//! # commentaire
//! G003 = {subject}: `{key}` refers to node "{node}", which does not exist
//! ```
//!
//! La clé d'un refus commence par son code stable (`G003`), suivi au besoin
//! d'une variante (`J005.zero`). Les autres clés désignent des fragments
//! (`subject.node`, `type.integer`). Dans un modèle, `{nom}` est remplacé par
//! l'argument de ce nom, le nom ne comptant que des minuscules, des chiffres et
//! des traits d'union ; toute autre accolade est rendue telle quelle. Les deux
//! caractères `\n` y commencent une nouvelle ligne.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// Ce qui remplit un emplacement d'un modèle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    /// Rendu tel quel, sans traduction.
    Text(String),
    /// Traduit à son tour, dans la même langue.
    Msg(Message),
}

/// Une phrase à composer : sa clé dans le catalogue, et ses arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub key: &'static str,
    pub args: Vec<(&'static str, Arg)>,
}

impl Message {
    /// Un nombre et son nom, « 1 node » ou « 3 nodes » : la clé `one` pour 1,
    /// la clé `other` sinon. C'est la règle anglaise du pluriel ; une deuxième
    /// langue apportera la sienne.
    pub fn count(number: usize, one: &'static str, other: &'static str) -> Message {
        Message::new(if number == 1 { one } else { other }).text("n", number)
    }

    pub fn new(key: &'static str) -> Message {
        Message {
            key,
            args: Vec::new(),
        }
    }

    /// Ajoute un argument rendu tel quel.
    pub fn text(mut self, name: &'static str, value: impl ToString) -> Message {
        self.args.push((name, Arg::Text(value.to_string())));
        self
    }

    /// Ajoute un argument qui est lui-même un message.
    pub fn msg(mut self, name: &'static str, value: Message) -> Message {
        self.args.push((name, Arg::Msg(value)));
        self
    }

    /// Compose la phrase dans la langue du catalogue.
    ///
    /// Une clé absente du catalogue ne fait pas échouer l'affichage : la clé et
    /// ses arguments sont alors rendus bruts, ce qui reste lisible et signale
    /// l'entrée manquante.
    pub fn render(&self, catalog: &Catalog) -> String {
        let Some(template) = catalog.get(self.key) else {
            let args: Vec<String> = self
                .args
                .iter()
                .map(|(name, arg)| format!("{name}={}", render_arg(arg, catalog)))
                .collect();
            return format!("[{}] {}", self.key, args.join(" "));
        };
        let mut out = String::new();
        for piece in pieces(template) {
            match piece {
                Piece::Literal(text) => out.push_str(text),
                Piece::Slot(name) => match self.args.iter().find(|(n, _)| *n == name) {
                    Some((_, arg)) => out.push_str(&render_arg(arg, catalog)),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                },
            }
        }
        out
    }

    /// Vérifie que le message s'accorde avec le catalogue : sa clé et celles
    /// des messages qu'il contient existent, et chaque modèle reçoit exactement
    /// les arguments qu'il nomme.
    pub fn check(&self, catalog: &Catalog) -> Result<(), String> {
        let Some(template) = catalog.get(self.key) else {
            return Err(format!("key `{}` missing from the catalogue", self.key));
        };
        let wanted: BTreeSet<&str> = pieces(template)
            .into_iter()
            .filter_map(|piece| match piece {
                Piece::Slot(name) => Some(name),
                Piece::Literal(_) => None,
            })
            .collect();
        let given: BTreeSet<&str> = self.args.iter().map(|(name, _)| *name).collect();
        if wanted != given || given.len() != self.args.len() {
            let names: Vec<&str> = self.args.iter().map(|(name, _)| *name).collect();
            return Err(format!(
                "key `{}`: the template wants {wanted:?}, the message gives {names:?}",
                self.key
            ));
        }
        for (_, arg) in &self.args {
            if let Arg::Msg(inner) = arg {
                inner.check(catalog)?;
            }
        }
        Ok(())
    }

    /// Toutes les clés employées : la sienne, puis celles de ses arguments.
    pub fn keys(&self) -> Vec<&'static str> {
        let mut keys = vec![self.key];
        for (_, arg) in &self.args {
            if let Arg::Msg(inner) = arg {
                keys.extend(inner.keys());
            }
        }
        keys
    }
}

fn render_arg(arg: &Arg, catalog: &Catalog) -> String {
    match arg {
        Arg::Text(text) => text.clone(),
        Arg::Msg(inner) => inner.render(catalog),
    }
}

/// Les modèles d'une langue, par clé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    language: &'static str,
    entries: BTreeMap<String, String>,
}

/// Catalogue anglais, langue par défaut, embarqué dans la bibliothèque.
const ENGLISH: &str = include_str!("../messages/en.txt");

impl Catalog {
    /// Le catalogue anglais.
    pub fn english() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| match Catalog::parse("en", ENGLISH) {
            Ok(catalog) => catalog,
            // Inatteignable tant que le test du catalogue passe ; un catalogue
            // vide laisse quand même l'affichage lisible, par clé et arguments.
            Err(_) => Catalog {
                language: "en",
                entries: BTreeMap::new(),
            },
        })
    }

    /// Code de la langue, au sens de BCP 47 : `en`.
    pub fn language(&self) -> &'static str {
        self.language
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Toutes les clés du catalogue, dans l'ordre.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Lit un catalogue. Le refus nomme la ligne fautive ; il s'adresse à
    /// qui écrit le catalogue, pas à l'utilisateur de l'outil.
    pub(crate) fn parse(language: &'static str, text: &str) -> Result<Catalog, String> {
        let mut entries = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, template)) = line.split_once(" = ") else {
                return Err(format!("line {number}: `key = template` expected"));
            };
            let valid = !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
            if !valid {
                return Err(format!("line {number}: invalid key `{key}`"));
            }
            if template.is_empty() || template.ends_with(char::is_whitespace) {
                return Err(format!(
                    "line {number}: empty template, or trailing whitespace"
                ));
            }
            if entries
                .insert(key.to_string(), template.replace("\\n", "\n"))
                .is_some()
            {
                return Err(format!("line {number}: key `{key}` already defined"));
            }
        }
        Ok(Catalog { language, entries })
    }
}

enum Piece<'a> {
    Literal(&'a str),
    Slot(&'a str),
}

/// Découpe un modèle en morceaux littéraux et en emplacements `{nom}`.
fn pieces(template: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = rest.get(open + 1..).unwrap_or("");
        let name_len = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        let is_slot = name_len > 0 && after.get(name_len..name_len + 1) == Some("}");
        if is_slot {
            out.push(Piece::Literal(rest.get(..open).unwrap_or("")));
            out.push(Piece::Slot(after.get(..name_len).unwrap_or("")));
            rest = after.get(name_len + 1..).unwrap_or("");
        } else {
            out.push(Piece::Literal(rest.get(..=open).unwrap_or("")));
            rest = after;
        }
    }
    out.push(Piece::Literal(rest));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRENCH: &str = "\
# Un catalogue d'essai, pour montrer qu'une autre langue suffit à changer la phrase.
G003 = {subject} : « {key} » désigne le nœud « {node} », qui n'existe pas
subject.edge = lien « {id} »
";

    fn sample() -> Message {
        Message::new("G003")
            .msg("subject", Message::new("subject.edge").text("id", "e1"))
            .text("key", "target")
            .text("node", "B")
    }

    #[test]
    fn the_english_catalogue_reads_without_error() {
        let catalog = Catalog::parse("en", ENGLISH).unwrap();
        assert!(catalog.keys().count() > 34);
        assert_eq!(Catalog::english().language(), "en");
    }

    #[test]
    fn a_message_renders_in_the_language_of_the_catalogue() {
        let french = Catalog::parse("fr", FRENCH).unwrap();
        assert_eq!(
            sample().render(Catalog::english()),
            "edge \"e1\": `target` refers to node \"B\", which does not exist"
        );
        assert_eq!(
            sample().render(&french),
            "lien « e1 » : « target » désigne le nœud « B », qui n'existe pas"
        );
        assert!(sample().check(&french).is_ok());
        assert_eq!(sample().keys(), ["G003", "subject.edge"]);
    }

    #[test]
    fn a_missing_key_still_renders_readably() {
        let empty = Catalog::parse("xx", "").unwrap();
        assert_eq!(
            sample().render(&empty),
            "[G003] subject=[subject.edge] id=e1 key=target node=B"
        );
        assert!(sample().check(&empty).is_err());
    }

    #[test]
    fn check_compares_slots_and_arguments() {
        let catalog = Catalog::parse("en", "K = {a} and {b}\n").unwrap();
        assert!(
            Message::new("K")
                .text("a", 1)
                .text("b", 2)
                .check(&catalog)
                .is_ok()
        );
        assert!(Message::new("K").text("a", 1).check(&catalog).is_err());
        assert!(
            Message::new("K")
                .text("a", 1)
                .text("b", 2)
                .text("c", 3)
                .check(&catalog)
                .is_err()
        );
        assert!(
            Message::new("K")
                .text("a", 1)
                .text("b", 2)
                .text("b", 3)
                .check(&catalog)
                .is_err()
        );
        // Un emplacement sans argument reste visible au lieu de disparaître.
        assert_eq!(Message::new("K").text("a", 1).render(&catalog), "1 and {b}");
    }

    #[test]
    fn only_lowercase_names_between_braces_are_slots() {
        let catalog = Catalog::parse("en", "K = `{` or `}`, {Upper}, {}, {x y}, {ok}\n").unwrap();
        assert_eq!(
            Message::new("K").text("ok", "!").render(&catalog),
            "`{` or `}`, {Upper}, {}, {x y}, !"
        );
    }

    #[test]
    fn a_bad_catalogue_is_refused_with_its_line() {
        let refusal = |text: &str| Catalog::parse("xx", text).unwrap_err();
        assert_eq!(refusal("# ok\nK: v\n"), "line 2: `key = template` expected");
        assert_eq!(refusal("K K = v\n"), "line 1: invalid key `K K`");
        assert_eq!(
            refusal("K = v \n"),
            "line 1: empty template, or trailing whitespace"
        );
        assert_eq!(refusal("K = v\nK = w\n"), "line 2: key `K` already defined");
    }

    #[test]
    fn backslash_n_starts_a_new_line() {
        let catalog = Catalog::parse("en", "K = one\\ntwo, and \\u stays\n").unwrap();
        assert_eq!(catalog.get("K"), Some("one\ntwo, and \\u stays"));
    }
}
