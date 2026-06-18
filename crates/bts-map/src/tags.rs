//! Tag extraction from source files using tree-sitter queries.
//!
//! Reuses the exact API pattern proven in SPIKE-bts-map.md:
//! - Most grammar crates export `LANGUAGE: LanguageFn` constant (use `.into()`)
//! - tree-sitter-typescript has TWO constants: LANGUAGE_TYPESCRIPT (.ts) and LANGUAGE_TSX (.tsx)
//! - npezza93-tree-sitter-swift uses the old `language()` fn API (ABI 0.22.x pattern)
//! - `QueryMatches` implements `StreamingIterator`, NOT `std::Iterator`
//! - Capture-name prefix routing: `name.definition.*` → Def, `name.reference.*` → Ref
// Items are used in #[cfg(test)] blocks and by render.rs, but the binary main() doesn't
// call them directly, so dead_code fires. Allow it at module level.
#![allow(dead_code)]

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

use crate::error::MapError;

// Vendored .scm queries — one per language.
const RUST_SCM: &str = include_str!("../vendor/queries/rust-tags.scm");
const PYTHON_SCM: &str = include_str!("../vendor/queries/python-tags.scm");
const TYPESCRIPT_SCM: &str = include_str!("../vendor/queries/typescript-tags.scm");
const JAVASCRIPT_SCM: &str = include_str!("../vendor/queries/javascript-tags.scm");
const GO_SCM: &str = include_str!("../vendor/queries/go-tags.scm");
const JAVA_SCM: &str = include_str!("../vendor/queries/java-tags.scm");
const RUBY_SCM: &str = include_str!("../vendor/queries/ruby-tags.scm");
const C_SCM: &str = include_str!("../vendor/queries/c-tags.scm");
const CPP_SCM: &str = include_str!("../vendor/queries/cpp-tags.scm");
const SWIFT_SCM: &str = include_str!("../vendor/queries/swift-tags.scm");

/// Which grammar to use when parsing a file.
///
/// Derived from file extension by `Lang::from_ext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    /// TypeScript (.ts); use `TypeScriptX` for .tsx
    TypeScript,
    /// TypeScript with JSX (.tsx)
    TypeScriptX,
    JavaScript,
    Go,
    Java,
    Ruby,
    C,
    Cpp,
    Swift,
}

impl Lang {
    /// Infer language from a file extension. Returns `None` for unsupported extensions.
    pub fn from_ext(path: &str) -> Option<Lang> {
        // Extract the final extension (handles "foo.c", "src/bar.rs", etc.)
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())?;
        match ext {
            "rs" => Some(Lang::Rust),
            "py" => Some(Lang::Python),
            "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::TypeScriptX),
            "js" | "mjs" | "cjs" => Some(Lang::JavaScript),
            "go" => Some(Lang::Go),
            "java" => Some(Lang::Java),
            "rb" => Some(Lang::Ruby),
            "c" | "h" => Some(Lang::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Lang::Cpp),
            "swift" => Some(Lang::Swift),
            _ => None,
        }
    }

    /// Human-readable name for diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "Rust",
            Lang::Python => "Python",
            Lang::TypeScript => "TypeScript",
            Lang::TypeScriptX => "TypeScriptX",
            Lang::JavaScript => "JavaScript",
            Lang::Go => "Go",
            Lang::Java => "Java",
            Lang::Ruby => "Ruby",
            Lang::C => "C",
            Lang::Cpp => "C++",
            Lang::Swift => "Swift",
        }
    }
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
///
/// Swift uses the old `language()` fn API (npezza93-tree-sitter-swift =0.4.4);
/// all others use the `LANGUAGE` constant pattern from tree-sitter 0.24.x.
pub fn language_for(lang: Lang) -> Language {
    match lang {
        Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
        Lang::Python => tree_sitter_python::LANGUAGE.into(),
        Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Lang::TypeScriptX => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Lang::Go => tree_sitter_go::LANGUAGE.into(),
        Lang::Java => tree_sitter_java::LANGUAGE.into(),
        Lang::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Lang::C => tree_sitter_c::LANGUAGE.into(),
        Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        // npezza93-tree-sitter-swift uses the legacy fn API, not the LANGUAGE constant.
        // The returned Language value is accepted by Parser::set_language at runtime.
        Lang::Swift => npezza93_tree_sitter_swift::language(),
    }
}

/// Return the vendored tag query source for a given `Lang`.
pub fn scm_for(lang: Lang) -> &'static str {
    match lang {
        Lang::Rust => RUST_SCM,
        Lang::Python => PYTHON_SCM,
        Lang::TypeScript | Lang::TypeScriptX => TYPESCRIPT_SCM,
        Lang::JavaScript => JAVASCRIPT_SCM,
        Lang::Go => GO_SCM,
        Lang::Java => JAVA_SCM,
        Lang::Ruby => RUBY_SCM,
        Lang::C => C_SCM,
        Lang::Cpp => CPP_SCM,
        Lang::Swift => SWIFT_SCM,
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
