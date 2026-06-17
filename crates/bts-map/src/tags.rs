//! Tag extraction from source files using tree-sitter queries.
//!
//! Reuses the exact API pattern proven in SPIKE-bts-map.md:
//! - Grammar crates export `LANGUAGE: LanguageFn` constant (use `.into()`)
//! - `QueryMatches` implements `StreamingIterator`, NOT `std::Iterator`
//! - Capture-name prefix routing: `name.definition.*` → Def, `name.reference.*` → Ref
#![allow(dead_code)]

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

use crate::error::MapError;

const RUST_SCM: &str = include_str!("../vendor/queries/rust-tags.scm");

/// Which grammar to use when parsing a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
}

/// Whether a tag identifies a definition site or a reference/call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Def,
    Ref,
}

/// A single named tag extracted from a source file.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Relative file path (used as graph node key).
    pub rel_fname: String,
    /// The identifier text (e.g. function name).
    pub name: String,
    /// 0-based line number in the source.
    pub line: usize,
    /// Definition or reference.
    pub kind: TagKind,
    /// The tree-sitter node kind of the captured node (e.g. "identifier").
    /// Exposed so tests can assert the correct-anchor vintage (spike d10).
    pub node_kind: String,
}

/// Resolve the tree-sitter `Language` for a given `Lang`.
pub fn language_for(lang: Lang) -> Language {
    match lang {
        Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
    }
}

/// Return the tag query source for a given `Lang`.
pub fn scm_for(lang: Lang) -> &'static str {
    match lang {
        Lang::Rust => RUST_SCM,
    }
}

/// Extract all `Tag`s from `source` for the given `lang`.
///
/// Returns a `MapError::QueryCompile` if the vendored `.scm` fails to compile
/// against the pinned grammar (should never happen with the spike-proven pins).
pub fn extract_tags(rel_fname: &str, source: &str, lang: Lang) -> Result<Vec<Tag>, MapError> {
    let language = language_for(lang);

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| MapError::QueryCompile(e.to_string()))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| MapError::ParseFailed(rel_fname.to_string()))?;

    let root = tree.root_node();

    let query = Query::new(&language, scm_for(lang))
        .map_err(|e| MapError::QueryCompile(format!("{:?}", e)))?;

    let mut cursor = QueryCursor::new();
    // tree-sitter 0.24.x: QueryMatches is StreamingIterator, NOT std::Iterator
    let mut matches = cursor.matches(&query, root, source.as_bytes());

    let mut tags = Vec::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_name = &query.capture_names()[cap.index as usize];
            let kind = if cap_name.starts_with("name.definition.") {
                TagKind::Def
            } else if cap_name.starts_with("name.reference.") {
                TagKind::Ref
            } else {
                // definition.* / reference.* surrounding nodes — skip for naming
                continue;
            };

            let node = cap.node;
            let line = node.start_position().row;
            let name = node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .to_string();
            let node_kind = node.kind().to_string();

            tags.push(Tag {
                rel_fname: rel_fname.to_string(),
                name,
                line,
                kind,
                node_kind,
            });
        }
    }

    Ok(tags)
}
