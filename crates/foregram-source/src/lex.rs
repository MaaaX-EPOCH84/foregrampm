// ForegramPM — lexer of the source format: tokens with their positions.
// Copyright (C) 2026 Maxime MARCONATO (MaaaX / EPOCH84)
// SPDX-License-Identifier: GPL-3.0-or-later

use foregram_graph::{Diagnostic, Message, Pos};

/// What the lexer cuts the text into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tok {
    /// A run of characters that are neither blank nor punctuation.
    Word(String),
    /// A string, decoded, quotes removed.
    Str(String),
    /// `{`
    Open,
    /// `}`
    Close,
    /// `;`
    Semi,
    /// The end of a line, `LF` or `CRLF`.
    Eol,
    /// A comment: its text after `//`, trailing blanks removed.
    Comment(String),
}

/// A token and the position of its first character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) tok: Tok,
    pub(crate) pos: Pos,
}

/// The tokens of a text and the refusals met on the way.
#[derive(Debug, Default)]
pub(crate) struct Lexed {
    pub(crate) tokens: Vec<Token>,
    pub(crate) diags: Vec<Diagnostic>,
}

/// The text of a file: UTF-8, without its byte order mark. Invalid bytes
/// stop the reading at the first of them.
pub(crate) fn decode(bytes: &[u8]) -> Result<&str, Diagnostic> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.strip_prefix('\u{FEFF}').unwrap_or(text)),
        Err(error) => {
            let valid = bytes.get(..error.valid_up_to()).unwrap_or(&[]);
            let prefix = std::str::from_utf8(valid).unwrap_or("");
            let prefix = prefix.strip_prefix('\u{FEFF}').unwrap_or(prefix);
            Err(Diagnostic::new(end_of(prefix), Message::new("SRC001")))
        }
    }
}

/// Cuts a decoded text into tokens. A lexical refusal does not stop the
/// lexer: the faulty character or escape is reported and skipped, an
/// unclosed string ends with its line, so the braces after it still count.
pub(crate) fn lex(text: &str) -> Lexed {
    let mut lexer = Lexer {
        rest: text.chars(),
        line: 1,
        col: 1,
        control_line: 0,
        out: Lexed::default(),
    };
    lexer.run();
    lexer.out
}

/// Position of the character that would follow `text`.
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

/// Whether `c` ends a word.
fn ends_word(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '{' | '}' | ';' | '"') || c.is_control()
}

struct Lexer<'a> {
    rest: std::str::Chars<'a>,
    line: u32,
    col: u32,
    /// The last line where a control character was reported: one per line.
    control_line: u32,
    out: Lexed,
}

impl Lexer<'_> {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.rest.clone().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut ahead = self.rest.clone();
        ahead.next();
        ahead.next()
    }

    /// Whether a line ends here: `LF`, or `CR` followed by `LF`.
    fn at_eol(&self) -> bool {
        match self.peek() {
            Some('\n') => true,
            Some('\r') => self.peek2() == Some('\n'),
            _ => false,
        }
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

    fn push(&mut self, tok: Tok, pos: Pos) {
        self.out.tokens.push(Token { tok, pos });
    }

    fn refuse(&mut self, pos: Pos, message: Message) {
        self.out.diags.push(Diagnostic::new(pos, message));
    }

    /// A control character other than tab and a line end: refused, at most
    /// once per line, then skipped as if it were blank.
    fn control(&mut self, pos: Pos, c: char) {
        if self.control_line != pos.line {
            self.control_line = pos.line;
            self.refuse(
                pos,
                Message::new("SRC002").text("char", format!("U+{:04X}", c as u32)),
            );
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let pos = self.pos();
            match c {
                ' ' | '\t' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    self.push(Tok::Eol, pos);
                }
                '\r' => {
                    self.bump();
                    if self.peek() == Some('\n') {
                        self.bump();
                        self.push(Tok::Eol, pos);
                    } else {
                        self.control(pos, c);
                    }
                }
                '{' => {
                    self.bump();
                    self.push(Tok::Open, pos);
                }
                '}' => {
                    self.bump();
                    self.push(Tok::Close, pos);
                }
                ';' => {
                    self.bump();
                    self.push(Tok::Semi, pos);
                }
                '"' => self.string(pos),
                '/' if self.peek2() == Some('/') => self.comment(pos),
                c if c.is_control() => {
                    self.bump();
                    self.control(pos, c);
                }
                _ => self.word(pos),
            }
        }
    }

    fn word(&mut self, pos: Pos) {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if ends_word(c) {
                break;
            }
            self.bump();
            text.push(c);
        }
        self.push(Tok::Word(text), pos);
    }

    /// A comment is a token like any other, so that the parser attaches it
    /// to the item it stands with.
    fn comment(&mut self, pos: Pos) {
        self.bump();
        self.bump();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if self.at_eol() {
                break;
            }
            let here = self.pos();
            self.bump();
            if c.is_control() && c != '\t' {
                self.control(here, c);
            } else {
                text.push(c);
            }
        }
        let trimmed = text.trim_end_matches([' ', '\t']).len();
        text.truncate(trimmed);
        self.push(Tok::Comment(text), pos);
    }

    fn string(&mut self, pos: Pos) {
        self.bump();
        let mut text = String::new();
        loop {
            if self.at_eol() {
                self.refuse(pos, Message::new("SRC003"));
                break;
            }
            let Some(c) = self.peek() else {
                self.refuse(pos, Message::new("SRC003"));
                break;
            };
            let here = self.pos();
            match c {
                '"' => {
                    self.bump();
                    break;
                }
                '\\' => self.escape(&mut text),
                c if c.is_control() && c != '\t' => {
                    self.bump();
                    self.control(here, c);
                }
                c => {
                    self.bump();
                    text.push(c);
                }
            }
        }
        self.push(Tok::Str(text), pos);
    }

    /// An escape inside a string, from its backslash. A backslash before
    /// the line end or a control character is left to the string, which
    /// reports what follows.
    fn escape(&mut self, text: &mut String) {
        let pos = self.pos();
        self.bump();
        if self.at_eol() {
            return;
        }
        let Some(c) = self.peek() else {
            return;
        };
        match c {
            '"' | '\\' => {
                self.bump();
                text.push(c);
            }
            'n' => {
                self.bump();
                text.push('\n');
            }
            't' => {
                self.bump();
                text.push('\t');
            }
            'u' => self.unicode(pos, text),
            c if c.is_control() => {}
            c => {
                self.bump();
                self.refuse(pos, Message::new("SRC004").text("escape", format!("\\{c}")));
            }
        }
    }

    /// `\u{H…}`: one to six hexadecimal digits naming a Unicode scalar value.
    fn unicode(&mut self, pos: Pos, text: &mut String) {
        self.bump();
        let mut written = String::from("\\u");
        let mut digits = String::new();
        let mut closed = false;
        if self.peek() == Some('{') {
            self.bump();
            written.push('{');
            while let Some(c) = self.peek().filter(char::is_ascii_hexdigit) {
                self.bump();
                digits.push(c);
                written.push(c);
            }
            if self.peek() == Some('}') {
                self.bump();
                written.push('}');
                closed = true;
            }
        }
        if !closed || digits.is_empty() || digits.len() > 6 {
            self.refuse(pos, Message::new("SRC004").text("escape", written));
            return;
        }
        match u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
        {
            Some(c) => text.push(c),
            None => self.refuse(pos, Message::new("SRC005").text("escape", written)),
        }
    }
}
