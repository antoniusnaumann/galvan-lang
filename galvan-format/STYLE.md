# The Galvan formatting style

Every style decision `galvan-format` makes, with the chosen default and the
precedent it follows. The formatter is deliberately low-configuration
(rustfmt/gofmt philosophy): only the indent unit and line width are
adjustable; everything else is canonical style.

## Layout basics

| Decision | Choice | Precedent |
| --- | --- | --- |
| Indentation | 4 spaces (`--indent-width`, `--use-tabs` to override) | Rust, Swift |
| Maximum line width | 100 columns, best effort (`--line-width`) | rustfmt's default |
| Trailing whitespace | always removed | everyone |
| Final newline | exactly one; empty files stay empty | everyone |
| Blank lines | preserved where the author put them, capped at one | rustfmt (`blank_lines_upper_bound = 1`), Prettier |
| Semicolons | removed; statements separated by `;` are split onto their own lines | idiomatic Galvan is newline-terminated |

## Spacing

- Single space around binary operators, `=` and compound assignments, and
  after `,` and `:` — never before them: `let x: Int = a + b`,
  `f(a: 1, b: 2)` (Rust, Swift, JS).
- Range operators are tight, like Rust/Swift ranges: `0..=5`, `0..<10`,
  `0..+n`. Exception: the tolerance range reads like arithmetic and keeps
  spaces: `100 ± 5` / `100 +- 5`.
- Member access is tight (`a.b?.c!`), `::` is tight (`Color::Red`,
  `serde_json::to_string`), postfix `!` and `?` are tight (`T?`,
  `value!`, `Result!Error`).
- No padding inside brackets of any kind: `(a, b)`, `[1, 2]`, `{1, 2}`,
  `{String: Int}`, `Vec<Int>` (Rust; unlike Prettier's `{ a }` object
  padding — Galvan braces are also set/dict literals, where padding would
  fight the type syntax).
- Empty dict/ordered-dict literals are `{:}` and `[:]`; empty everything
  else collapses: `()`, `[]`, `{}`.
- Closure parameter pipes hug their parameters: `|name|`, `|a, b|`, with a
  space before the opening pipe and after the closing one:
  `for xs |x| { ... }` (matches all hand-written Galvan).
- One space before every `{` that opens a body or type declaration:
  `fn f() {`, `type Dog {` (Rust, Go, Swift, JS).

## Line breaking

- **Bodies always break** — one statement per line, even single-statement
  `if`/`for`/`try`/`else` bodies (rustfmt; Galvan's control flow is
  trailing-closure syntax, so this also keeps `if` and closures visually
  consistent). Two exceptions stay on one line when they fit and hold a
  single statement:
  - match arms: `Blue { foo() }` (the established Galvan corpus style),
  - closure bodies: `|x| { x + 1 }`.
- **Bracketed lists** (call arguments, parameters, collection literals,
  tuple/variant fields) fit on one line, or break with one element per
  line, indented once, with a **trailing comma** — and collapse back when
  they fit (rustfmt, Prettier):

  ```galvan
  let response = TicketResponse(
      id: id,
      title: title,
  )
  ```

- **Member chains** that fit stay flat (`a.b.c`). Overlong chains break
  before each `.`, indented once; when the receiver is a bare
  identifier/type name the first link stays on the base line (Prettier's
  first-segment heuristic, and how the axum example is hand-written):

  ```galvan
  let app = router.route("/health", get(health))
      .route("/tickets", get(list_tickets))
      .with_state(state)
  ```

- A comment inside a bracketed list forces the broken layout (Prettier) —
  doc comments on `cmd` parameters keep their own lines.
- Free-function argument lists (`println "x"`, statement calls without
  parentheses) are never reflowed — there is no bracket to anchor a
  continuation line, and the grammar forbids a trailing comma there.
- Binary/infix expressions are never broken at the operator (deferred;
  long expressions keep their length until parenthesized-group breaking is
  worth the complexity).

## What the formatter never touches

- **Token spellings**: Unicode operator variants (`≠`, `≥`, `→`, `±`),
  keyword synonyms (`and` vs `&&`) and number formats stay exactly as
  written — normalizing them is a linter's job.
- **String contents**, including interpolations: `"a  \( x+1 )  b"` is
  emitted byte-for-byte.
- **Comment text** (only placement is managed: own-line comments keep
  their own line at the current indent; a comment trailing code stays on
  that line, one space before `//`).
- **`use` declaration order** — not sorted (yet); reordering user lines
  with attached comments is riskier than the win. Revisit alongside an
  import-organizing code action.
- **Files with syntax errors** — formatting refuses entirely rather than
  rearranging code around a broken parse (`gofmt`; unlike Prettier, which
  refuses too, but very unlike editors' whitespace formatters).

## Invariants

- Idempotent: `format(format(x)) == format(x)` (enforced by every test).
- Loss-free: comments, blank-line paragraphs and token spellings survive.
- The example projects under `example-projects/` are formatted in exactly
  this style and are checked by tests in both `galvan-format` and
  `galvan-lsp`.
