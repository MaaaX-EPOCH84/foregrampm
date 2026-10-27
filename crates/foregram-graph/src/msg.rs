// ForegramPM — localisable messages: a key, arguments, a catalogue per language
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! A message does not carry its sentence: it carries a key and arguments, and
//! the sentence is composed at display time from the catalogue of the chosen
//! language. An argument is either a text rendered as is (an identifier, a
//! value), or another message, translated in turn ('node "A"',
//! '`width` (node)').
//!
//! A catalogue is a text file, one entry per line:
//!
//! ```text
//! # comment
//! GPH003 = {subject}: `{key}` refers to node "{node}", which does not exist
//! ```
//!
//! The key of a refusal starts with its stable code (`GPH003`), followed where
//! needed by a variant (`JSN005.zero`). The other keys name fragments
//! (`subject.node`, `type.integer`). In a template, `{name}` is replaced by the
//! argument of that name, a name being made only of lowercase letters, digits
//! and hyphens; any other brace is rendered as is. The two characters `\n`
//! start a new line there.
//!
//! A number followed by its noun ('3 nodes') has two entries, `key.one` and
//! `key.other`. The message carries only the base key and the number: the
//! variant is chosen at display time, by the rule of the language. English
//! takes `one` for 1, French for 0 and 1.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::json::quote;

/// What fills a slot of a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    /// Rendered as is, without translation.
    Text(String),
    /// Translated in turn, into the same language.
    Msg(Message),
}

/// A sentence to compose: its key in the catalogue, and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The key of the template in the catalogue (the base key for a plural).
    pub key: &'static str,
    /// The arguments, as `(name, value)` pairs, in the order given.
    pub args: Vec<(&'static str, Arg)>,
    /// The number that picks the `.one` or `.other` variant of a plural key;
    /// `None` for an ordinary key.
    pub count: Option<usize>,
}

impl Message {
    /// A number and its noun, '1 node' or '3 nodes': `key` is the base key,
    /// without `.one` or `.other`, and the number is the argument `n`. The
    /// variant is chosen at display time, by the rule of the language.
    pub fn count(number: usize, key: &'static str) -> Message {
        let mut message = Message::new(key).text("n", number);
        message.count = Some(number);
        message
    }

    /// A message for `key`, with no argument and no count.
    pub fn new(key: &'static str) -> Message {
        Message {
            key,
            args: Vec::new(),
            count: None,
        }
    }

    /// Adds an argument rendered as is.
    pub fn text(mut self, name: &'static str, value: impl ToString) -> Message {
        self.args.push((name, Arg::Text(value.to_string())));
        self
    }

    /// Adds an argument that is itself a message.
    pub fn msg(mut self, name: &'static str, value: Message) -> Message {
        self.args.push((name, Arg::Msg(value)));
        self
    }

    /// Composes the sentence in the language of the catalogue.
    ///
    /// A key missing from the catalogue does not make the display fail: the
    /// key and its arguments are then rendered raw, which stays readable and
    /// flags the missing entry.
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

    /// Checks that the message agrees with the catalogue: its key and those of
    /// the messages it contains exist, and each template receives exactly the
    /// arguments it names. A plural key is checked in both of its variants,
    /// whatever the number.
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

    /// The message as JSON, on one line: `{"key": …, "args": {…}}`. Without a
    /// sentence: it is composed once, at the top of the object that carries
    /// it. A plural key is written there without a variant, the same in every
    /// language; the number is in `n`.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"key\": {}, \"args\": {}}}",
            quote(self.key),
            self.args_json()
        )
    }

    /// The arguments as a JSON object, in the order they were given: a string
    /// for a text, `{"key": …, "args": {…}}` for a message.
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

    /// Every key used: its own, then those of its arguments. A plural key
    /// appears there without a variant.
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

/// The templates of a language, by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    language: &'static str,
    entries: BTreeMap<String, String>,
}

/// The English catalogue, the default language, embedded in the library.
const ENGLISH: &str = include_str!("../messages/en.txt");
/// The French catalogue, embedded too. It carries only the product keys.
const FRENCH: &str = include_str!("../messages/fr.txt");

impl Catalog {
    /// The languages with an embedded catalogue, as BCP 47 codes.
    pub const LANGUAGES: &'static [&'static str] = &["en", "fr"];

    /// The English catalogue, the default language.
    pub fn english() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| Catalog::embedded("en", ENGLISH))
    }

    /// The French catalogue.
    pub fn french() -> &'static Catalog {
        static CATALOG: OnceLock<Catalog> = OnceLock::new();
        CATALOG.get_or_init(|| Catalog::embedded("fr", FRENCH))
    }

    /// The catalogue of a language, by its code (`en`, `fr`); `None` for a
    /// language without a catalogue.
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
            // Unreachable as long as the catalogue test passes; an empty
            // catalogue still leaves the display readable, by key and
            // arguments.
            Err(_) => Catalog {
                language,
                entries: BTreeMap::new(),
            },
        }
    }

    /// The variant of a plural key for this number, `one` or `other`, by the
    /// rule of the language: singular for 1 in English, for 0 and 1 in French.
    fn plural(&self, number: usize) -> &'static str {
        let singular = match self.language {
            "fr" => number <= 1,
            _ => number == 1,
        };
        if singular { "one" } else { "other" }
    }

    /// The language code, as a BCP 47 code: `en`, `fr`.
    pub fn language(&self) -> &'static str {
        self.language
    }

    /// The template for `key`, or `None` if the catalogue has no such entry.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Every key of the catalogue, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Reads a catalogue. The refusal names the faulty line; it is meant for
    /// whoever writes the catalogue, not for the user of the tool.
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

/// The names of the slots of a template.
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

/// Splits a template into literal pieces and `{name}` slots.
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
GPH003 = {subject} : « {key} » désigne le nœud « {node} », qui n'existe pas
subject.edge = lien « {id} »
";

    fn sample() -> Message {
        Message::new("GPH003")
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
        assert_eq!(sample().keys(), ["GPH003", "subject.edge"]);
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
        // The key stays the base key, in JSON as in the list of keys.
        assert_eq!(
            Message::count(0, "K").to_json(),
            "{\"key\": \"K\", \"args\": {\"n\": \"0\"}}"
        );
        assert_eq!(Message::count(0, "K").keys(), ["K"]);
        // Both variants are checked, whatever the number.
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
            "[GPH003] subject=[subject.edge] id=e1 key=target node=B"
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
        // A slot without an argument stays visible instead of vanishing.
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
