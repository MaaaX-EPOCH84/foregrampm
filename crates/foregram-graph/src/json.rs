// ForegramPM — lecture stricte de JSON, avec la position de chaque valeur.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sous-ensemble strict de JSON (RFC 8259).
//!
//! Trois écarts volontaires avec la norme, tous des refus :
//! - les nombres sont des entiers, sans partie décimale ni exposant ;
//! - une clé ne paraît qu'une fois dans un objet ;
//! - le texte est en UTF-8 sans marque d'ordre d'octets.
//!
//! Chaque valeur et chaque clé gardent leur position, pour qu'un refus
//! prononcé plus tard, sur le sens, désigne encore l'endroit du texte.

use crate::diag::{Diagnostic, Pos};

/// Profondeur d'imbrication admise. Les formats de ForegramPM en emploient cinq.
pub const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    pub pos: Pos,
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Array(Vec<Value>),
    Object(Vec<Member>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub key: String,
    pub key_pos: Pos,
    pub value: Value,
}

impl Kind {
    /// Nom du type, tel qu'il paraît dans un message.
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Null => "null",
            Kind::Bool(_) => "boolean",
            Kind::Int(_) => "integer",
            Kind::Str(_) => "string",
            Kind::Array(_) => "array",
            Kind::Object(_) => "object",
        }
    }
}

/// Lit un texte JSON entier. Le premier défaut arrête la lecture.
pub fn parse(bytes: &[u8]) -> Result<Value, Diagnostic> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid = bytes.get(..error.valid_up_to()).unwrap_or(&[]);
            let prefix = std::str::from_utf8(valid).unwrap_or("");
            return Err(Diagnostic::new(
                "J001",
                end_of(prefix),
                "invalid bytes: the text must be UTF-8",
            ));
        }
    };
    if text.starts_with('\u{FEFF}') {
        return Err(Diagnostic::new(
            "J001",
            Pos { line: 1, col: 1 },
            "byte order mark (BOM) refused at the start of the text",
        ));
    }
    let mut parser = Parser {
        rest: text.chars(),
        line: 1,
        col: 1,
    };
    parser.skip_ws();
    let value = parser.value(0)?;
    parser.skip_ws();
    let pos = parser.pos();
    if let Some(c) = parser.peek() {
        return Err(Diagnostic::new(
            "J010",
            pos,
            format!("unexpected content after the value: `{}`", show(c)),
        ));
    }
    Ok(value)
}

/// Écrit une chaîne JSON, guillemets compris, sous sa forme canonique :
/// seuls les guillemets, la barre oblique inverse et les caractères de
/// contrôle sont échappés.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Écrit `"clé": [ … ]` avec un élément par ligne, forme canonique des listes.
pub(crate) fn push_list(out: &mut String, key: &str, lines: &[String], last: bool) {
    out.push_str(&format!("  {}: [", quote(key)));
    if !lines.is_empty() {
        out.push('\n');
        for (i, line) in lines.iter().enumerate() {
            out.push_str("    ");
            out.push_str(line);
            if i + 1 < lines.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ");
    }
    out.push(']');
    if !last {
        out.push(',');
    }
    out.push('\n');
}

/// Position du caractère qui suivrait `text`.
fn end_of(text: &str) -> Pos {
    let mut pos = Pos { line: 1, col: 1 };
    for c in text.chars() {
        if c == '\n' {
            pos.line = pos.line.saturating_add(1);
            pos.col = 1;
        } else {
            pos.col = pos.col.saturating_add(1);
        }
    }
    pos
}

/// Un caractère tel qu'un message peut le montrer.
fn show(c: char) -> String {
    if c.is_control() {
        format!("U+{:04X}", c as u32)
    } else {
        c.to_string()
    }
}

struct Parser<'a> {
    rest: std::str::Chars<'a>,
    line: u32,
    col: u32,
}

impl Parser<'_> {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.rest.clone().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.rest.next()?;
        if c == '\n' {
            self.line = self.line.saturating_add(1);
            self.col = 1;
        } else {
            self.col = self.col.saturating_add(1);
        }
        Some(c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.bump();
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Diagnostic> {
        let pos = self.pos();
        let kind = match self.peek() {
            None => {
                return Err(Diagnostic::new(
                    "J003",
                    pos,
                    "unexpected end of text: a value is expected",
                ));
            }
            Some('{') => {
                check_depth(depth, pos)?;
                self.object(depth)?
            }
            Some('[') => {
                check_depth(depth, pos)?;
                self.array(depth)?
            }
            Some('"') => Kind::Str(self.string()?),
            Some(c) if c == '-' || c.is_ascii_digit() => Kind::Int(self.number()?),
            Some(c) if c.is_ascii_alphabetic() => self.word()?,
            Some(c) => {
                return Err(Diagnostic::new(
                    "J002",
                    pos,
                    format!("unexpected character `{}`: a value is expected", show(c)),
                ));
            }
        };
        Ok(Value { pos, kind })
    }

    fn word(&mut self) -> Result<Kind, Diagnostic> {
        let pos = self.pos();
        let mut word = String::new();
        while let Some(c) = self.peek() {
            if !c.is_ascii_alphanumeric() {
                break;
            }
            word.push(c);
            self.bump();
        }
        match word.as_str() {
            "true" => Ok(Kind::Bool(true)),
            "false" => Ok(Kind::Bool(false)),
            "null" => Ok(Kind::Null),
            _ => Err(Diagnostic::new(
                "J002",
                pos,
                format!("unknown word `{word}`: a value is expected"),
            )),
        }
    }

    fn number(&mut self) -> Result<i64, Diagnostic> {
        let pos = self.pos();
        let negative = self.peek() == Some('-');
        if negative {
            self.bump();
        }
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            digits.push(c);
            self.bump();
        }
        if digits.is_empty() {
            return Err(Diagnostic::new(
                "J005",
                pos,
                "malformed number: a digit is expected after `-`",
            ));
        }
        if matches!(self.peek(), Some('.' | 'e' | 'E')) {
            return Err(Diagnostic::new(
                "J004",
                pos,
                "non-integer number: only integers are allowed",
            ));
        }
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(Diagnostic::new(
                "J005",
                pos,
                "malformed number: leading zero",
            ));
        }
        if negative && digits == "0" {
            return Err(Diagnostic::new(
                "J005",
                pos,
                "malformed number: `-0` is written `0`",
            ));
        }
        let mut value: i64 = 0;
        for digit in digits.bytes() {
            let digit = i64::from(digit - b'0');
            let next = value.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(digit)
                } else {
                    v.checked_add(digit)
                }
            });
            value = next.ok_or_else(|| {
                Diagnostic::new("J006", pos, "integer too large to be represented")
            })?;
        }
        Ok(value)
    }

    fn string(&mut self) -> Result<String, Diagnostic> {
        let start = self.pos();
        self.bump();
        let mut out = String::new();
        loop {
            let pos = self.pos();
            match self.bump() {
                None => {
                    return Err(Diagnostic::new("J003", start, "unterminated string"));
                }
                Some('"') => return Ok(out),
                Some('\\') => out.push(self.escape(pos)?),
                Some(c) if (c as u32) < 0x20 => {
                    return Err(Diagnostic::new(
                        "J007",
                        pos,
                        format!("unescaped control character {} in a string", show(c)),
                    ));
                }
                Some(c) => out.push(c),
            }
        }
    }

    fn escape(&mut self, pos: Pos) -> Result<char, Diagnostic> {
        let bad = |message: &str| Diagnostic::new("J007", pos, message);
        match self.bump() {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('/') => Ok('/'),
            Some('b') => Ok('\u{8}'),
            Some('f') => Ok('\u{c}'),
            Some('n') => Ok('\n'),
            Some('r') => Ok('\r'),
            Some('t') => Ok('\t'),
            Some('u') => {
                let high = self.hex4(pos)?;
                if (0xD800..0xDC00).contains(&high) {
                    if self.bump() != Some('\\') || self.bump() != Some('u') {
                        return Err(bad("high surrogate without its low surrogate"));
                    }
                    let low = self.hex4(pos)?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(bad("high surrogate without its low surrogate"));
                    }
                    let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                    char::from_u32(code).ok_or_else(|| bad("invalid code point"))
                } else if (0xDC00..0xE000).contains(&high) {
                    Err(bad("lone low surrogate"))
                } else {
                    char::from_u32(high).ok_or_else(|| bad("invalid code point"))
                }
            }
            _ => Err(bad("unknown escape sequence")),
        }
    }

    fn hex4(&mut self, pos: Pos) -> Result<u32, Diagnostic> {
        let mut code = 0;
        for _ in 0..4 {
            let digit = self.bump().and_then(|c| c.to_digit(16)).ok_or_else(|| {
                Diagnostic::new(
                    "J007",
                    pos,
                    "four hexadecimal digits are expected after `\\u`",
                )
            })?;
            code = code * 16 + digit;
        }
        Ok(code)
    }

    fn array(&mut self, depth: usize) -> Result<Kind, Diagnostic> {
        self.bump();
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Kind::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            let pos = self.pos();
            match self.bump() {
                Some(',') => {}
                Some(']') => return Ok(Kind::Array(items)),
                other => return Err(expected(other, pos, "`,` or `]`")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Kind, Diagnostic> {
        self.bump();
        let mut members: Vec<Member> = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Kind::Object(members));
        }
        loop {
            self.skip_ws();
            let key_pos = self.pos();
            if self.peek() != Some('"') {
                return Err(expected(self.peek(), key_pos, "a quoted key"));
            }
            let key = self.string()?;
            if members.iter().any(|m| m.key == key) {
                return Err(Diagnostic::new(
                    "J008",
                    key_pos,
                    format!("duplicate key `{key}` in the same object"),
                ));
            }
            self.skip_ws();
            let pos = self.pos();
            match self.bump() {
                Some(':') => {}
                other => return Err(expected(other, pos, "`:`")),
            }
            self.skip_ws();
            let value = self.value(depth + 1)?;
            members.push(Member {
                key,
                key_pos,
                value,
            });
            self.skip_ws();
            let pos = self.pos();
            match self.bump() {
                Some(',') => {}
                Some('}') => return Ok(Kind::Object(members)),
                other => return Err(expected(other, pos, "`,` or `}`")),
            }
        }
    }
}

fn check_depth(depth: usize, pos: Pos) -> Result<(), Diagnostic> {
    if depth >= MAX_DEPTH {
        return Err(Diagnostic::new(
            "J009",
            pos,
            format!("nesting too deep: at most {MAX_DEPTH} levels"),
        ));
    }
    Ok(())
}

fn expected(found: Option<char>, pos: Pos, what: &str) -> Diagnostic {
    match found {
        None => Diagnostic::new(
            "J003",
            pos,
            format!("unexpected end of text: {what} expected"),
        ),
        Some(c) => Diagnostic::new(
            "J002",
            pos,
            format!("unexpected character `{}`: {what} expected", show(c)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refusal(text: &str) -> (&'static str, u32, u32) {
        let d = parse(text.as_bytes()).unwrap_err();
        (d.code, d.pos.line, d.pos.col)
    }

    #[test]
    fn reads_every_kind_and_keeps_positions() {
        let v = parse(b"{\n  \"a\": [1, -2, true, false, null, \"x\"]\n}").unwrap();
        let Kind::Object(members) = &v.kind else {
            panic!("objet attendu")
        };
        assert_eq!(v.pos, Pos { line: 1, col: 1 });
        assert_eq!(members[0].key, "a");
        assert_eq!(members[0].key_pos, Pos { line: 2, col: 3 });
        let Kind::Array(items) = &members[0].value.kind else {
            panic!("tableau attendu")
        };
        assert_eq!(items[0].kind, Kind::Int(1));
        assert_eq!(items[1].kind, Kind::Int(-2));
        assert_eq!(items[1].pos, Pos { line: 2, col: 12 });
        assert_eq!(items[2].kind, Kind::Bool(true));
        assert_eq!(items[3].kind, Kind::Bool(false));
        assert_eq!(items[4].kind, Kind::Null);
        assert_eq!(items[5].kind, Kind::Str("x".into()));
    }

    #[test]
    fn empty_text_and_blank_text_are_refused() {
        assert_eq!(refusal(""), ("J003", 1, 1));
        assert_eq!(refusal(" \n\t "), ("J003", 2, 3));
    }

    #[test]
    fn numbers_must_be_plain_integers() {
        assert_eq!(refusal("1.5"), ("J004", 1, 1));
        assert_eq!(refusal("[1e3]"), ("J004", 1, 2));
        assert_eq!(refusal("01"), ("J005", 1, 1));
        assert_eq!(refusal("-"), ("J005", 1, 1));
        assert_eq!(refusal("-0"), ("J005", 1, 1));
        assert_eq!(refusal("9223372036854775808"), ("J006", 1, 1));
        assert_eq!(refusal("-9223372036854775809"), ("J006", 1, 1));
    }

    #[test]
    fn integer_bounds_are_read_exactly() {
        assert_eq!(
            parse(b"9223372036854775807").unwrap().kind,
            Kind::Int(i64::MAX)
        );
        assert_eq!(
            parse(b"-9223372036854775808").unwrap().kind,
            Kind::Int(i64::MIN)
        );
        assert_eq!(parse(b"0").unwrap().kind, Kind::Int(0));
    }

    #[test]
    fn duplicate_keys_are_refused_at_the_second_key() {
        assert_eq!(refusal("{\"a\": 1,\n \"a\": 2}"), ("J008", 2, 2));
    }

    #[test]
    fn structure_errors_name_the_place() {
        assert_eq!(refusal("[1,]"), ("J002", 1, 4));
        assert_eq!(refusal("[1 2]"), ("J002", 1, 4));
        assert_eq!(refusal("{\"a\" 1}"), ("J002", 1, 6));
        assert_eq!(refusal("{a: 1}"), ("J002", 1, 2));
        assert_eq!(refusal("[1"), ("J003", 1, 3));
        assert_eq!(refusal("{\"a\": 1"), ("J003", 1, 8));
        assert_eq!(refusal("tru"), ("J002", 1, 1));
        assert_eq!(refusal("truex"), ("J002", 1, 1));
        assert_eq!(refusal("?"), ("J002", 1, 1));
        assert_eq!(refusal("1 2"), ("J010", 1, 3));
        assert_eq!(refusal("{} x"), ("J010", 1, 4));
    }

    #[test]
    fn strings_decode_escapes_and_surrogate_pairs() {
        let v = parse(br#""a\"b\\c\/d\n\u00e9\ud83d\ude00""#).unwrap();
        assert_eq!(v.kind, Kind::Str("a\"b\\c/d\né😀".into()));
    }

    #[test]
    fn bad_strings_are_refused() {
        assert_eq!(refusal("\"abc"), ("J003", 1, 1));
        assert_eq!(refusal("\"a\tb\""), ("J007", 1, 3));
        assert_eq!(refusal(r#""\x""#), ("J007", 1, 2));
        assert_eq!(refusal(r#""\u12g4""#), ("J007", 1, 2));
        assert_eq!(refusal(r#""\ud83d""#), ("J007", 1, 2));
        assert_eq!(refusal(r#""\ude00""#), ("J007", 1, 2));
        assert_eq!(refusal(r#""\ud83d\u0041""#), ("J007", 1, 2));
    }

    #[test]
    fn columns_count_code_points_not_bytes() {
        // « é » et « 😀 » tiennent sur plusieurs octets mais une seule colonne.
        assert_eq!(refusal("[\"é😀\", ?]"), ("J002", 1, 8));
    }

    #[test]
    fn carriage_return_line_feed_counts_one_line() {
        assert_eq!(refusal("[\r\n?]"), ("J002", 2, 1));
    }

    #[test]
    fn invalid_utf8_and_bom_are_refused() {
        let d = parse(b"[\n \xff]").unwrap_err();
        assert_eq!((d.code, d.pos.line, d.pos.col), ("J001", 2, 2));
        let d = parse(b"\xef\xbb\xbf{}").unwrap_err();
        assert_eq!((d.code, d.pos.line, d.pos.col), ("J001", 1, 1));
    }

    #[test]
    fn nesting_is_bounded() {
        let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(parse(ok.as_bytes()).is_ok());
        let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
        let d = parse(deep.as_bytes()).unwrap_err();
        assert_eq!(d.code, "J009");
        assert_eq!(d.pos.col as usize, MAX_DEPTH + 1);
    }

    #[test]
    fn quote_escapes_only_what_it_must() {
        assert_eq!(quote("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
        let round = parse(quote("a\"b\\c\n\u{1}é/😀").as_bytes()).unwrap();
        assert_eq!(round.kind, Kind::Str("a\"b\\c\n\u{1}é/😀".into()));
    }
}
