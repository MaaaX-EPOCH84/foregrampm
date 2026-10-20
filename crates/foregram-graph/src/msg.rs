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
//!
//! Un nombre suivi de son nom (« 3 nodes ») a deux entrées, `clé.one` et
//! `clé.other`. Le message ne porte que la clé de base et le nombre : la
//! variante se choisit à l'affichage, selon la règle de la langue — l'anglais
//! prend `one` pour 1, le français pour 0 et 1.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::json::quote;

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
    /// Le nombre qui choisit la variante `.one` ou `.other` d'une clé de
    /// pluriel ; `None` pour une clé ordinaire.
    pub count: Option<usize>,
}

impl Message {
    /// Un nombre et son nom, « 1 node » ou « 3 nodes » : `key` est la clé de
    /// base, sans `.one` ni `.other`, et le nombre est l'argument `n`. La
    /// variante se choisit à l'affichage, par la règle de la langue.
    pub fn count(number: usize, key: &'static str) -> Message {
        let mut message = Message::new(key).text("n", number);
        message.count = Some(number);
        message
    }

    pub fn new(key: &'static str) -> Message {
        Message {
            key,
            args: Vec::new(),
            count: None,
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
        let template = match self.count {
            None => catalog.get(self.key),
            Some(number) => catalog.get(&format!("{}.{}", self.key, catalog.plural(number))),
        };
        let Some(template) = template else {
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
    /// les arguments qu'il nomme. Une clé de pluriel est vérifiée dans ses
    /// deux variantes, quel que soit le nombre.
    pub fn check(&self, catalog: &Catalog) -> Result<(), String> {
        let keys = match self.count {
            None => vec![self.key.to_string()],
            Some(_) => vec![format!("{}.one", self.key), format!("{}.other", self.key)],
        };
        let given: BTreeSet<&str> = self.args.iter().map(|(name, _)| *name).collect();
        for key in keys {
            let Some(template) = catalog.get(&key) else {
                return Err(format!("key `{key}` missing from the catalogue"));
            };
            let wanted = slots(template);
            if wanted != given || given.len() != self.args.len() {
                let names: Vec<&str> = self.args.iter().map(|(name, _)| *name).collect();
                return Err(format!(
                    "key `{key}`: the template wants {wanted:?}, the message gives {names:?}"
                ));
            }
        }
        for (_, arg) in &self.args {
            if let Arg::Msg(inner) = arg {
                inner.check(catalog)?;
            }
        }
        Ok(())
    }

    /// Le message en JSON, sur une ligne : `{"key": …, "args": {…}}`. Sans
    /// phrase : elle se compose une fois, au sommet de l'objet qui le porte.
    /// Une clé de pluriel s'y écrit sans variante, la même dans toutes les
    /// langues ; le nombre est dans `n`.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"key\": {}, \"args\": {}}}",
            quote(self.key),
            self.args_json()
        )
    }

    /// Les arguments en un objet JSON, dans l'ordre où ils ont été donnés :
    /// une chaîne pour un texte, `{"key": …, "args": {…}}` pour un message.
    pub fn args_json(&self) -> String {
        let args: Vec<String> = self
            .args
            .iter()
            .map(|(name, arg)| {
                let value = match arg {
                    Arg::Text(text) => quote(text),
                    Arg::Msg(inner) => inner.to_json(),
                };
                format!("{}: {value}", quote(name))
            })
            .collect();
        format!("{{{}}}", args.join(", "))
    }

    /// Toutes les clés employées : la sienne, puis celles de ses arguments.
    /// Une clé de pluriel y figure sans variante.
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
/// Catalogue français, embarqué aussi. Il ne porte que les clés du produit.
const FRENCH: &str = include_str!("../messages/fr.txt");

impl Catalog {
    /// Les langues dont un catalogue est embarqué, au sens de BCP 47.
    pub const LANGUAGES: &'static [&'static str] = &["en", "fr"];

    /// Le catalogue anglais, langue par défaut.
    pub fn english() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| Catalog::embedded("en", ENGLISH))
    }

    /// Le catalogue français.
    pub fn french() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| Catalog::embedded("fr", FRENCH))
    }

    /// Le catalogue d'une langue, par son code (`en`, `fr`) ; `None` pour
    /// une langue sans catalogue.
    pub fn for_language(code: &str) -> Option<&'static Catalog> {
        match code {
            "en" => Some(Catalog::english()),
            "fr" => Some(Catalog::french()),
            _ => None,
        }
    }

    fn embedded(language: &'static str, text: &str) -> Catalog {
        match Catalog::parse(language, text) {
            Ok(catalog) => catalog,
            // Inatteignable tant que le test du catalogue passe ; un catalogue
            // vide laisse quand même l'affichage lisible, par clé et arguments.
            Err(_) => Catalog {
                language,
                entries: BTreeMap::new(),
            },
        }
    }

    /// La variante d'une clé de pluriel pour ce nombre, `one` ou `other`,
    /// selon la règle de la langue : le singulier pour 1 en anglais, pour 0
    /// et 1 en français.
    fn plural(&self, number: usize) -> &'static str {
        let singular = match self.language {
            "fr" => number <= 1,
            _ => number == 1,
        };
        if singular { "one" } else { "other" }
    }

    /// Code de la langue, au sens de BCP 47 : `en`, `fr`.
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

/// Les noms des emplacements d'un modèle.
fn slots(template: &str) -> BTreeSet<&str> {
    pieces(template)
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Slot(name) => Some(name),
            Piece::Literal(_) => None,
        })
        .collect()
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

    const SAMPLE: &str = "\
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
        let french = Catalog::parse("fr", SAMPLE).unwrap();
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
    fn the_plural_variant_follows_the_language_of_the_catalogue() {
        let text = "K.one = {n} thing\nK.other = {n} things\n";
        let english = Catalog::parse("en", text).unwrap();
        let french = Catalog::parse("fr", text).unwrap();
        let said = |n: usize, catalog: &Catalog| Message::count(n, "K").render(catalog);
        assert_eq!(
            [0, 1, 2].map(|n| said(n, &english)),
            ["0 things", "1 thing", "2 things"]
        );
        assert_eq!(
            [0, 1, 2].map(|n| said(n, &french)),
            ["0 thing", "1 thing", "2 things"]
        );
        // La clé reste la base, en JSON comme dans la liste des clés.
        assert_eq!(
            Message::count(0, "K").to_json(),
            "{\"key\": \"K\", \"args\": {\"n\": \"0\"}}"
        );
        assert_eq!(Message::count(0, "K").keys(), ["K"]);
        // Les deux variantes sont vérifiées, quel que soit le nombre.
        assert!(Message::count(1, "K").check(&english).is_ok());
        let half = Catalog::parse("en", "K.one = {n} thing\n").unwrap();
        assert!(Message::count(1, "K").check(&half).is_err());
        let mismatch = Catalog::parse("en", "K.one = {n} thing\nK.other = {m} things\n").unwrap();
        assert!(Message::count(1, "K").check(&mismatch).is_err());
    }

    #[test]
    fn a_language_code_finds_its_catalogue() {
        assert_eq!(Catalog::LANGUAGES, ["en", "fr"]);
        for code in Catalog::LANGUAGES {
            assert_eq!(Catalog::for_language(code).unwrap().language(), *code);
        }
        assert!(Catalog::for_language("de").is_none());
        assert!(Catalog::for_language("EN").is_none());
        assert!(Catalog::parse("fr", FRENCH).is_ok());
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
