# Source format of a diagram

A diagram is written in one text file, read by the compiler and by the
agent that drives it, and edited by hand. This document fixes the
**grammar** of that file and the **document model** it yields; it fixes
nothing about what the elements mean. Meaning belongs to the **notation**
the file declares — the Merise conceptual data model first, UML later —
and the notation's rules live in their own documents.

**One file is one drawing.** `gestpret.fg` becomes `gestpret.svg`.
Several views of the same elements, and elements shared between files,
are not described here; nothing in the grammar prevents them.

The file extension is `.fg`.

Three parties read the file, and the grammar serves all three:

- **the agent**, which needs a strict format: a refusal always carries a
  line and a column, and the same source always yields the same bytes;
- **the human**, who writes and reads it by hand, so the file stays close
  to prose: one line per fact, no quoting unless a label needs it;
- **the editor** to come, which rewrites the file in canonical form after
  every gesture, so the canonical form must lose nothing — not even a
  comment.

The reader is written by hand, with positions on every token and every
node from the first version; numbers are kept as text; lengths and
columns count code points; every refusal of a file is collected and
reported, sorted by position; the reader never panics. These rules
follow the lessons of an earlier parser of the same author, where
positions had to be retrofitted token by token.

## Text

- **UTF-8**, with or without a byte order mark, which is dropped. Any
  other encoding is refused at the first offending byte.
- **Lines** end with `LF` or `CRLF`; the canonical form writes `LF`.
- **Positions** are a line and a column, both from 1, the column in
  code points — the same convention as the exchange format.
- **Whitespace** is the space and the tab, between tokens. Indentation
  carries no meaning: blocks are delimited by braces, and the canonical
  form indents by two spaces per level.
- **Control characters** other than tab and the line ends are refused;
  a `CR` that is not followed by `LF` is one.

## Tokens

| Token | Form |
|---|---|
| **word** | a maximal run of characters that are neither whitespace nor `{`, `}`, `;` nor `"`, and that does not start `//` |
| **string** | `"…"` on one line, with the escapes `\"`, `\\`, `\n`, `\t` and `\u{H…}` (one to six hexadecimal digits, a Unicode scalar value) |
| **punctuation** | `{`, `}`, `;` |
| **end of line** | significant: it ends a directive, an element head or a content line |
| **comment** | `//` to the end of the line, kept by the canonical form |

A word is text and stays text: `0,n`, `#id_membre`, `12.5` and `MEMBRE`
are words, and the format converts none of them. A notation that wants
a number parses the word itself and refuses it with the word's position.

Comments are written `//`, not `#`: in the seed of the Merise notation a
`#` marks an identifier (`#id_membre`), and a content line must be able
to start with it.

## Structure

```
file       = header { element | ';' | eol }
header     = { directive | ';' | eol }
directive  = '@' word { value } end
element    = kind id [ label ] '{' body '}'
body       = { item }
item       = element | attribute | content | ';' | eol
attribute  = '@' word { value } end
content    = value { value } end
value      = word | string
end        = eol | ';'
```

where `kind` and `id` are words, `label` is a string, `@` followed by a
word is one word starting with `@`, and blank lines and comments may
appear anywhere a token may.

**Values followed by `{` are an element head; values that start with a
word starting with `@` are a directive or an attribute; any other values
are content.** An item ends at the end of its line, at a `;`, or before
a `}`. A line usually holds one item, so the rule reads line by line: a
line with `{` is an element head, a line starting with `@` a directive or
an attribute, any other line content. Nothing else tells them apart, and
in particular the format does not know the kinds of the notation: `entity`, `association`, `group`, or any other
word, parses the same way. The notation decides which kinds exist and
what their content means.

### Header

The header is the directives before the first element: what the file as
a whole declares.

```
@notation merise
@direction right
```

A directive is `@`, a key, then zero or more values. In this version of
the format every value is opaque: the format keeps the words and strings
with their positions and interprets none of them. The notation reads
`@notation`; the layout and the renderer will read the directives that
pin a drawing — direction, window, ratio — when they exist. A directive
after the first element is refused: the header is in one place. A key
given twice in the header is refused.

### Element

```
entity MATERIEL "Matériel prêté" {
  #id_materiel
  designation
  @layer 0
}
```

An element is a **kind**, an **identifier**, an optional **label**, and a
block.

- The **kind** is a word, meaningful to the notation only.
- The **identifier** names the element for every reader: a reference in
  another element, a refusal, the agent that asks for a change, the
  editor that applies one. It is made of ASCII letters, digits and `_`,
  starts with a letter or `_`, and is case-sensitive. Accents and spaces
  belong to the label. An identifier is **unique in the whole file**,
  nesting included: the second definition is refused, with the position
  of the first.
- The **label** is a string, the text the drawing shows when it differs
  from the identifier; without a label the identifier is shown. Only a
  string may stand between the identifier and the brace.
- The **block** is mandatory, opened on the head line and closed by `}`
  on any later line, or on the same one. An empty block is `{}`.

The head is the tokens before `{` on its line: the kind, the identifier
and the label, in that order, and nothing else is accepted there.

### Body

The block holds **items**, in the author's order, which the format
keeps:

- a **child element**, with the same grammar, which makes its parent a
  container — `group VENTES "Ventes" { entity CLIENT { … } }`;
- an **attribute**, `@` then a key and values, read by the format or by
  the layout, never by the notation: `@layer 1`;
- a **content line**, one or more values that the notation reads and
  the format keeps as a list of tokens: `#id_membre`, `nom`,
  `MEMBRE 0,n`.

The end of the line ends an attribute or a content line; `;` ends one on
the same line, so that `entity CLIENT { #id_client; nom }` is the same
element as its four-line form. A `}` also ends the item before it on the
same line. Nesting is limited to 32 levels of blocks; deeper is refused.

A key given twice in the same block is refused.

### What the format checks, and what it leaves

The format refuses what breaks the grammar above, and what breaks the
two rules it owns: the identifier's form and its uniqueness. It accepts
any kind, any directive key, any content: a file under an unknown
notation, or with no `@notation` at all, is **well-formed** when its
text parses, and that is all `foregram check` says of it in this
version.

Which content words name other elements is the notation's business —
in Merise, the first word of `MEMBRE 0,n` does. The format provides the
lookup, by identifier, that a notation uses to resolve them and to
refuse a reference to an element that does not exist.

## Document model

What a well-formed file becomes, and what the public API of the reader
returns:

- a **document**: its directives, then its top-level elements, in file
  order;
- a **directive**: key, values, position of the `@`;
- an **element**: kind, identifier, optional label, items in file
  order, position of the kind; each item is a child element, an
  attribute (key, values, position) or a content line (values,
  position of the first);
- a **value**: a word or a string, its text — the string decoded —, its
  position.

Every node carries the position of its first character, so that any
later refusal — by the notation, by the layout — points into the file
the author wrote; an element also carries the position of its
identifier and of its closing `}`. Comments are kept aside, in a list
beside the document, each with its text after `//` and its position,
for the canonical form to attach and write back; they are not part of
the model the notation reads. The layout of whitespace is not kept.

## Canonical form

`foregram canon` rewrites a source file in a fixed form: one item per
line, two spaces per level, one space between tokens, `LF`, comments
kept and attached to the item that follows them (or to the end of the
block), the author's order untouched, labels written only when they
differ from the identifier, strings escaped with the shortest escape.
Rewriting a canonical file yields the same bytes. The exact rules and
their evidence come with the canonical form itself.

## Refusals

A refusal carries a code of the family **`T`**, a position, and a
sentence from the catalogue, in English or in French, as text or as
JSON, exactly like the refusals of the exchange format. The reader
collects every refusal it can find in the file and reports them sorted
by position; a file with one refusal is not read further by the
notation, and the reader returns no partial document.

After a refusal the reader goes on, so that one pass reports what it
can:

- bytes that are not UTF-8 stop the reading: `T001` is then the only
  refusal;
- a control character is skipped as a blank, and refused once per line
  at most;
- a faulty escape is skipped; a string not closed ends with its line;
- a faulty item — a directive, an attribute, an element head — is
  dropped up to its end, the end of its line, a `;` or a `}`; a faulty
  element head still opens its block, so that the braces stay paired
  and the block is read for its own refusals;
- outside every element, a run of lines that are not element heads is
  refused once, at its first line;
- a block deeper than 32 levels is refused at its `{` and skipped
  without being read, its braces only counted;
- each block still open at the end of the file is refused at its `{`.

A duplicate identifier or key is refused at its second occurrence; the
sentence gives the position of the first.

| Code | Refuses |
|---|---|
| T001 | text that is not UTF-8 |
| T002 | a control character other than tab and the line ends, a lone `CR` included |
| T003 | a string not closed before the end of its line |
| T004 | an escape that is not `\"`, `\\`, `\n`, `\t` or `\u{H…}` |
| T005 | a `\u{…}` that is not a Unicode scalar value |
| T006 | a `}` with no open block |
| T007 | the end of the file inside a block, at the block's `{` |
| T008 | an element head that is not kind, identifier, optional label; outside every element, a line that is neither an element head nor, before the first element, a directive |
| T009 | an identifier with a character outside ASCII letters, digits and `_`, or starting with a digit |
| T010 | a directive after the first element |
| T011 | a `@` with no key |
| T012 | an identifier already defined, with the position of the first definition |
| T013 | a directive or attribute key given twice in the same header or block |
| T014 | more than 32 nested blocks |

The codes are reserved here and produced by the reader; the
documentation of the reader lists exactly the codes it produces, and a
test holds the two together.

## Example

GestPret, the seed's example model, in the source format:

```
// GestPret: members borrow equipment.
@notation merise
@direction right

entity MEMBRE {
  #id_membre
  nom
  prenom
  courriel
  date_adhesion
  actif
}

entity MATERIEL "Matériel" {
  #id_materiel
  #reference
  designation
  date_achat
  valeur_achat
  en_service
}

association PRET {
  date_pret
  date_retour_prevue
  date_retour_reelle
  MEMBRE 0,n
  MATERIEL 0,n
}
```

The same with a container and items on one line, which the canonical
form unfolds:

```
@notation merise

group PERSONNES "Personnes" {
  entity MEMBRE { #id_membre; nom; prenom }
}
entity MATERIEL { #id_materiel; designation }
association PRET { date_pret; MEMBRE 0,n; MATERIEL 0,n }
```

The four conceptual data models of the in-house corpus are written in
this format next to their seed form, under `corpus/mcd/`:
`gestpret.fg`, `commande.fg`, `employe.fg` and `interpreteur_wl.fg`.
