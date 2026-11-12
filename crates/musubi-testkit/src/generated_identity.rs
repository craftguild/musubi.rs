//! Closed identifiers accepted at generated release and Rust module boundaries.

pub(crate) fn valid_entry_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 256
        && bytes
            .next()
            .is_some_and(|byte| matches!(byte, b'_' | b'$') || byte.is_ascii_alphabetic())
        && bytes.all(|byte| matches!(byte, b'_' | b'$') || byte.is_ascii_alphanumeric())
}

pub(crate) fn valid_rust_module(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 256
        && bytes
            .next()
            .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && !rust_keyword(value)
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

fn rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "_" | "as"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}
