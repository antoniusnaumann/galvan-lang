//! Lowering of the tree-sitter concrete syntax tree into the [`Doc`] IR.
//!
//! Every emitter walks the *actual* children of its node (named and
//! anonymous), so comments — which tree-sitter attaches as extras anywhere in
//! the tree — are never dropped: structured emitters place them on their own
//! line or after the code they trail, and every other position falls back to
//! "space, comment, line break", which is always syntactically safe because
//! line comments run to the end of the line in the original source too.
//!
//! Token text is emitted verbatim, so Unicode operator spellings (`≠`, `→`),
//! raw strings and interpolations survive untouched.

use galvan_parse::Node;

use crate::doc::Doc;

pub fn source_doc(root: Node<'_>, src: &str) -> Doc {
    let ctx = Ctx { src };
    Doc::Concat(ctx.block_items(&collect(root), None, false))
}

struct Ctx<'a> {
    src: &'a str,
}

/// All non-empty children of a node, in source order. Semicolons are dropped
/// (statements are newline-separated in formatted output) and zero-width
/// tokens (the scanner's automatic semicolons) are skipped.
fn collect(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.end_byte() > child.start_byte())
        .filter(|child| child.kind() != ";")
        .collect()
}

fn is_comment(node: &Node<'_>) -> bool {
    node.kind() == "comment"
}

fn is_comma(node: &Node<'_>) -> bool {
    matches!(node.kind(), "," | "comma")
}

/// Kind of the first token a subtree renders.
fn first_leaf_kind(node: Node<'_>) -> &'static str {
    edge_kind(node, true)
}

/// Kind of the last token a subtree renders.
fn last_leaf_kind(node: Node<'_>) -> &'static str {
    edge_kind(node, false)
}

/// Descend to the outermost edge of the subtree, but stop at named
/// single-token rules: some grammar tokens (e.g. `brace_open`) contain an
/// anonymous child with the raw text (`{`) covering the same span, and the
/// spacing tables are keyed by the named kind.
fn edge_kind(node: Node<'_>, first: bool) -> &'static str {
    let mut current = node;
    loop {
        let count = current.child_count();
        if count == 0 {
            return current.kind();
        }
        let child = current
            .child(if first { 0 } else { count - 1 })
            .expect("child index in range");
        if !child.is_named()
            && child.start_byte() == current.start_byte()
            && child.end_byte() == current.end_byte()
        {
            return current.kind();
        }
        current = child;
    }
}

/// Tokens that no space follows (`(x`, `a.b`, `ns::name`, `0..=5`).
const TIGHT_AFTER: &[&str] = &[
    "paren_open",
    "bracket_open",
    "brace_open",
    "angle_bracket_open",
    "double_colon",
    "dot",
    "member_call_operator",
    "safe_call_operator",
    "inclusive_range",
    "exclusive_range",
    "interval_range",
];

/// Tokens that no space precedes (`x)`, `a,`, `name:`, `T?`, `x!`).
const TIGHT_BEFORE: &[&str] = &[
    "paren_close",
    "bracket_close",
    "brace_close",
    "angle_bracket_close",
    ",",
    "comma",
    "colon",
    "double_colon",
    "dot",
    "member_call_operator",
    "safe_call_operator",
    "question_mark",
    "exclamation_mark",
    "inclusive_range",
    "exclusive_range",
    "interval_range",
];

impl<'a> Ctx<'a> {
    fn text_of(&self, node: Node<'_>) -> &'a str {
        &self.src[node.start_byte()..node.end_byte()]
    }

    /// Verbatim token or token-like subtree (strings keep their exact bytes,
    /// including interpolations).
    fn verbatim(&self, node: Node<'_>) -> Doc {
        let text = self.text_of(node);
        if text.contains('\n') {
            Doc::RawText(text.to_owned())
        } else {
            Doc::Text(text.to_owned())
        }
    }

    fn newlines_between(&self, prev: Node<'_>, next: Node<'_>) -> usize {
        self.src[prev.end_byte()..next.start_byte()]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
    }

    /// Main dispatch.
    fn doc(&self, node: Node<'_>) -> Doc {
        match node.kind() {
            // Verbatim subtrees: their content is data, not structure.
            "comment" | "string_literal" | "raw_string_literal" | "char_literal"
            | "number_literal" => self.verbatim(node),

            "body" => self.body(node, false),
            "match_arm" => self.match_arm(node),
            "match_expression" | "struct" => self.braced_block(node),
            "enum" => self.braced_block(node),

            "function_call" | "constructor_call" | "enum_constructor" | "enum_variant"
            | "match_pattern_args" | "param_list" | "tuple_struct" | "tuple_type"
            | "parametric_type" => self.with_bracketed_list(node),

            "array_literal" | "ordered_dict_literal" => self.collection(node),
            "set_literal" | "dict_literal" => self.collection(node),

            "member_expression" | "argument_modifier_expression" => self.chain(node),
            "postfix_expression" | "result_type" | "enum_access" | "use_path" => {
                self.tight(&collect(node))
            }

            "closure" | "closure_type" => self.closure(node),
            "trailing_closure_expression" => self.trailing_closure(node),
            "else_expression" => self.else_expression(node),

            _ => {
                if node.child_count() == 0 {
                    self.verbatim(node)
                } else {
                    self.join(&collect(node))
                }
            }
        }
    }

    /// Generic child joiner: single space between children unless the
    /// adjacent tokens ask for tight spacing.
    fn join(&self, children: &[Node<'_>]) -> Doc {
        let mut parts = Vec::new();
        let mut prev: Option<Node<'_>> = None;
        for &child in children {
            if let Some(prev) = prev {
                if is_comment(&child) {
                    if self.newlines_between(prev, child) == 0 {
                        parts.push(Doc::text(" "));
                    } else {
                        parts.push(Doc::HardLine);
                    }
                } else if is_comment(&prev) || prev.kind() == "annotation" {
                    parts.push(Doc::HardLine);
                } else if pair_spaced(prev, child) {
                    parts.push(Doc::text(" "));
                }
            }
            parts.push(self.doc(child));
            prev = Some(child);
        }
        Doc::Concat(parts)
    }

    /// Children joined with no spaces at all (member chains fallback,
    /// `Result!Error`, `Type::Case`, `value!`).
    fn tight(&self, children: &[Node<'_>]) -> Doc {
        let mut parts = Vec::new();
        for (index, &child) in children.iter().enumerate() {
            if is_comment(&child) {
                parts.push(Doc::text(" "));
                parts.push(self.verbatim(child));
                if index + 1 != children.len() {
                    parts.push(Doc::HardLine);
                }
                continue;
            }
            parts.push(self.doc(child));
        }
        Doc::Concat(parts)
    }

    /// The statements/fields/arms between braces, each prefixed with its
    /// separator: a hard line break, a preserved blank line (capped at one),
    /// or — for a comment on the same line as the code it trails — a space.
    fn block_items(&self, inner: &[Node<'_>], terminator: Option<&str>, leading: bool) -> Vec<Doc> {
        let mut parts = Vec::new();
        let mut prev: Option<Node<'_>> = None;
        for &child in inner {
            if is_comma(&child) {
                continue;
            }
            match prev {
                None => {
                    if leading {
                        parts.push(Doc::HardLine);
                    }
                }
                Some(prev) => {
                    let gap = self.newlines_between(prev, child);
                    if is_comment(&child) && gap == 0 {
                        parts.push(Doc::text(" "));
                    } else if gap >= 2 {
                        parts.push(Doc::BlankLine);
                    } else {
                        parts.push(Doc::HardLine);
                    }
                }
            }
            parts.push(self.doc(child));
            if !is_comment(&child) {
                if let Some(terminator) = terminator {
                    parts.push(Doc::text(terminator));
                }
            }
            prev = Some(child);
        }
        parts
    }

    /// `{ ... }` statement body. Only match arms and closures may keep a
    /// single-statement body on one line (`Blue { foo() }`); all other bodies
    /// always break. An empty body collapses to `{}`.
    fn body(&self, node: Node<'_>, allow_inline: bool) -> Doc {
        let children = collect(node);
        let open = children
            .iter()
            .position(|child| child.kind() == "brace_open")
            .expect("body has an opening brace");
        let close = children
            .iter()
            .rposition(|child| child.kind() == "brace_close")
            .expect("body has a closing brace");
        let inner = &children[open + 1..close];

        if inner.is_empty() {
            return Doc::text("{}");
        }
        if allow_inline && inner.len() == 1 && !is_comment(&inner[0]) {
            return Doc::Group(vec![
                Doc::text("{"),
                Doc::Indent(vec![Doc::Line, self.doc(inner[0])]),
                Doc::Line,
                Doc::text("}"),
            ]);
        }
        Doc::Concat(vec![
            Doc::text("{"),
            Doc::Indent(self.block_items(inner, None, true)),
            Doc::HardLine,
            Doc::text("}"),
        ])
    }

    fn match_arm(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let mut parts = Vec::new();
        for (index, &child) in children.iter().enumerate() {
            if index > 0 {
                parts.push(Doc::text(" "));
            }
            if child.kind() == "body" {
                parts.push(self.body(child, true));
            } else {
                parts.push(self.doc(child));
            }
        }
        Doc::Concat(parts)
    }

    /// `prefix { items }` blocks that always break: structs, enums and match
    /// expressions. Struct fields and enum variants get a `,` terminator.
    fn braced_block(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let open = children
            .iter()
            .position(|child| child.kind() == "brace_open")
            .expect("braced block has an opening brace");
        let close = children
            .iter()
            .rposition(|child| child.kind() == "brace_close")
            .expect("braced block has a closing brace");

        let terminator = match node.kind() {
            "struct" | "enum" => Some(","),
            _ => None,
        };

        let mut parts = vec![self.join(&children[..open]), Doc::text(" ")];
        let inner = &children[open + 1..close];
        if inner.is_empty() {
            parts.push(Doc::text("{}"));
        } else {
            parts.push(Doc::text("{"));
            parts.push(Doc::Indent(self.block_items(inner, terminator, true)));
            parts.push(Doc::HardLine);
            parts.push(Doc::text("}"));
        }
        Doc::Concat(parts)
    }

    /// A node that is (or ends in) a bracketed, comma-separated list:
    /// calls, parameter lists, tuple structs/types, `Vec<T>`. Fits on one
    /// line without a trailing comma, or breaks one-element-per-line with
    /// one. A comment inside the brackets forces the broken form.
    fn with_bracketed_list(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let (open_kind, close_kind) = if node.kind() == "parametric_type" {
            ("angle_bracket_open", "angle_bracket_close")
        } else if children.iter().any(|child| child.kind() == "paren_open") {
            ("paren_open", "paren_close")
        } else {
            // Optional argument list absent (e.g. a bare enum variant).
            return self.join(&children);
        };

        let open = children
            .iter()
            .position(|child| child.kind() == open_kind)
            .expect("list has an opening bracket");
        let close = children
            .iter()
            .rposition(|child| child.kind() == close_kind)
            .expect("list has a closing bracket");

        let mut parts = Vec::new();
        if open > 0 {
            parts.push(self.join(&children[..open]));
        }
        parts.push(self.list(&children[open + 1..close], open_kind, close_kind));
        Doc::Concat(parts)
    }

    /// Collection literals: `[a, b]`, `{a, b}`, `{k: v}`, and the empty
    /// dict/ordered-dict spellings `{:}` / `[:]`.
    fn collection(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let inner = &children[1..children.len() - 1];
        let open = children[0].kind();
        let close = children[children.len() - 1].kind();

        if inner.len() == 1 && inner[0].kind() == "colon" {
            return Doc::text(format!("{}:{}", bracket_text(open), bracket_text(close)));
        }
        self.list(inner, open, close)
    }

    fn list(&self, inner: &[Node<'_>], open_kind: &str, close_kind: &str) -> Doc {
        let open = bracket_text(open_kind);
        let close = bracket_text(close_kind);
        let items: Vec<Node<'_>> = inner
            .iter()
            .copied()
            .filter(|child| !is_comma(child))
            .collect();

        if items.is_empty() {
            return Doc::text(format!("{open}{close}"));
        }
        if items.iter().any(is_comment) {
            // Comments force the broken layout; block_items places them.
            return Doc::Concat(vec![
                Doc::text(open),
                Doc::Indent(self.block_items(&items, Some(","), true)),
                Doc::HardLine,
                Doc::text(close),
            ]);
        }

        let mut list = vec![Doc::SoftLine];
        for (index, &item) in items.iter().enumerate() {
            if index > 0 {
                list.push(Doc::text(","));
                list.push(Doc::Line);
            }
            list.push(self.doc(item));
        }
        list.push(Doc::IfBreak(",".to_owned()));
        Doc::Group(vec![
            Doc::text(open),
            Doc::Indent(list),
            Doc::SoftLine,
            Doc::text(close),
        ])
    }

    /// Member chains. A chain hanging off a bare identifier keeps its first
    /// link on the base line (`Router.new()`); when the whole chain does not
    /// fit, the remaining links break one per line, indented once:
    ///
    /// ```text
    /// let app = Router.new()
    ///     .route("/health", get(health))
    ///     .with_state(state)
    /// ```
    fn chain(&self, node: Node<'_>) -> Doc {
        let mut links: Vec<(Node<'_>, Node<'_>)> = Vec::new();
        let mut current = node;
        let base = loop {
            let children = collect(current);
            if children.len() != 3 || children.iter().any(is_comment) {
                // Unexpected shape (usually an interleaved comment): fall
                // back to the generic joiner for the whole expression.
                return self.join(&collect(node));
            }
            links.push((children[1], children[2]));
            let inner = unwrap_expression(children[0]);
            match inner.map(|inner| inner.kind()) {
                Some("member_expression") | Some("argument_modifier_expression") => {
                    current = inner.unwrap();
                }
                _ => break children[0],
            }
        };
        links.reverse();

        let base_is_simple = matches!(
            unwrap_expression(base).map(|inner| inner.kind()),
            Some("ident") | Some("type_ident")
        );

        let mut head = vec![self.doc(base)];
        let rest = if base_is_simple {
            let (op, rhs) = links[0];
            head.push(self.doc(op));
            head.push(self.doc(rhs));
            &links[1..]
        } else {
            &links[..]
        };

        if rest.is_empty() {
            return Doc::Concat(head);
        }
        let mut tail = Vec::new();
        for &(op, rhs) in rest {
            tail.push(Doc::SoftLine);
            tail.push(self.doc(op));
            tail.push(self.doc(rhs));
        }
        head.push(Doc::Indent(tail));
        Doc::Group(head)
    }

    /// `|a, b|` parameter pipes: tight inside, then the closure result —
    /// an expression, a type (closure types) or a body that may stay inline.
    fn closure(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let mut parts = Vec::new();
        let mut index = 0;
        parts.push(self.pipe_section(&children, &mut index));
        while index < children.len() {
            let child = children[index];
            parts.push(Doc::text(" "));
            if child.kind() == "body" {
                parts.push(self.body(child, true));
            } else {
                parts.push(self.doc(child));
            }
            index += 1;
        }
        Doc::Concat(parts)
    }

    /// `ident args |params| { ... }` — `if cond { }`, `for xs |x| { }`,
    /// `try value |v| { }` all share this shape.
    fn trailing_closure(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let mut parts = vec![self.doc(children[0])];
        let mut index = 1;

        let mut first_argument = true;
        while index < children.len()
            && !matches!(children[index].kind(), "pipe" | "body")
        {
            let child = children[index];
            index += 1;
            if is_comma(&child) {
                continue;
            }
            parts.push(Doc::text(if first_argument { " " } else { ", " }));
            parts.push(self.doc(child));
            first_argument = false;
        }

        if index < children.len() && children[index].kind() == "pipe" {
            parts.push(Doc::text(" "));
            parts.push(self.pipe_section(&children, &mut index));
        }
        while index < children.len() {
            parts.push(Doc::text(" "));
            parts.push(self.body(children[index], false));
            index += 1;
        }
        Doc::Concat(parts)
    }

    /// `receiver else { ... }` / `receiver else |err| { ... }`. The body
    /// always breaks, matching the other control-flow bodies.
    fn else_expression(&self, node: Node<'_>) -> Doc {
        let children = collect(node);
        let mut parts = vec![self.doc(children[0])];
        let mut index = 1;
        while index < children.len() {
            let child = children[index];
            parts.push(Doc::text(" "));
            match child.kind() {
                "pipe" => {
                    parts.push(self.pipe_section(&children, &mut index));
                    continue;
                }
                "body" => parts.push(self.body(child, false)),
                _ => parts.push(self.doc(child)),
            }
            index += 1;
        }
        Doc::Concat(parts)
    }

    /// Consume `| a, b |` starting at `children[*index]` (an opening pipe)
    /// and render it as `|a, b|`. Leaves `*index` past the closing pipe.
    fn pipe_section(&self, children: &[Node<'_>], index: &mut usize) -> Doc {
        let mut parts = vec![Doc::text("|")];
        debug_assert_eq!(children[*index].kind(), "pipe");
        *index += 1;

        let mut first = true;
        while *index < children.len() {
            let child = children[*index];
            *index += 1;
            if child.kind() == "pipe" {
                break;
            }
            if is_comma(&child) {
                continue;
            }
            if is_comment(&child) {
                parts.push(Doc::text(" "));
                parts.push(self.verbatim(child));
                parts.push(Doc::HardLine);
                continue;
            }
            if !first {
                parts.push(Doc::text(", "));
            }
            parts.push(self.doc(child));
            first = false;
        }
        parts.push(Doc::text("|"));
        Doc::Concat(parts)
    }
}

/// The single expression wrapped by an `expression` node, if the node is
/// such a wrapper without interleaved comments.
fn unwrap_expression(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "expression" {
        return Some(node);
    }
    let children = collect(node);
    match children.as_slice() {
        [single] => Some(*single),
        _ => None,
    }
}

fn bracket_text(kind: &str) -> &'static str {
    match kind {
        "paren_open" => "(",
        "paren_close" => ")",
        "bracket_open" => "[",
        "bracket_close" => "]",
        "brace_open" => "{",
        "brace_close" => "}",
        "angle_bracket_open" => "<",
        "angle_bracket_close" => ">",
        other => panic!("not a bracket kind: {other}"),
    }
}

/// Whether a space separates two adjacent children, judged by the tokens
/// that actually touch (the last token of `prev`, the first of `next`).
fn pair_spaced(prev: Node<'_>, next: Node<'_>) -> bool {
    // Argument/parameter lists attach directly to what precedes them
    // (`fn name(...)`, `Rgb(r: red)`), unlike e.g. a parenthesized
    // expression passed to a free function (`println (a + b)`).
    if matches!(next.kind(), "param_list" | "match_pattern_args") {
        return false;
    }
    let prev = last_leaf_kind(prev);
    let next = first_leaf_kind(next);
    !TIGHT_AFTER.contains(&prev) && !TIGHT_BEFORE.contains(&next)
}
