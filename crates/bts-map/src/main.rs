#![deny(unsafe_code)]

mod error;
mod graph;
mod tags;

use anyhow::Result;
use clap::Parser;

/// bts-map — generate a ranked repo map from source tags.
///
/// Reads the current directory (or a specified path) and emits a ranked list
/// of source files, ordered by PageRank over the def/ref symbol graph.
/// Output is trimmed to the requested token BUDGET (approximate; default 1024).
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Token budget: maximum number of tokens in the ranked output (approximate).
    /// Files are included in PageRank order until the budget is exhausted.
    #[arg(default_value_t = 1024)]
    budget: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("bts-map: token budget = {}", cli.budget);
    println!("(full implementation in e05s03)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::graph::rank_files;
    use crate::tags::{extract_tags, Lang, TagKind};

    /// Fixture: a simple Rust file with both definitions and call-site references.
    const RUST_FIXTURE: &str = r#"
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

struct Greeter {
    prefix: String,
}

impl Greeter {
    fn new(prefix: &str) -> Self {
        Greeter { prefix: prefix.to_string() }
    }
    fn greet(&self, name: &str) -> String {
        format!("{} {}!", self.prefix, name)
    }
}

fn main() {
    let g = Greeter::new("Hello");
    println!("{}", g.greet("world"));
    greet("raw");
}
"#;

    /// e05s02 gate: defs>0 AND refs>0 AND def node kind=="identifier".
    ///
    /// The node-kind assertion catches the wrong-anchor vintage drift described
    /// in SPIKE-bts-map.md §"The aider dual-vintage structural drift":
    /// the older tree-sitter-languages vintage anchors @definition.method to
    /// `declaration_list` (kind="declaration_list"), not the function name node
    /// (kind="identifier"). A count-only check passes both vintages; this check
    /// does not.
    #[test]
    fn rust_tags() {
        let tags = extract_tags("src/lib.rs", RUST_FIXTURE, Lang::Rust)
            .expect("extract_tags should not fail with spike-proven pins");

        let defs: Vec<_> = tags.iter().filter(|t| t.kind == TagKind::Def).collect();
        let refs: Vec<_> = tags.iter().filter(|t| t.kind == TagKind::Ref).collect();

        assert!(
            !defs.is_empty(),
            "Expected >0 definition tags; got 0. Check .scm vendoring or grammar pin."
        );
        assert!(
            !refs.is_empty(),
            "Expected >0 reference tags; got 0. Check .scm vendoring or grammar pin."
        );

        // Spike mandatory gate (d10): method/function defs must be anchored to
        // an "identifier" node — NOT a container node like "declaration_list".
        //
        // The tree-sitter-language-pack vintage captures:
        //   `name: (identifier) @name.definition.method`  → node_kind == "identifier"
        //
        // The older tree-sitter-languages vintage (WRONG) captures:
        //   `(declaration_list ...) @definition.method`   → node_kind == "declaration_list"
        //
        // Class/struct defs legitimately capture `(type_identifier)` for
        // `name.definition.class` — we only check method/function defs here.
        let method_defs: Vec<_> = tags
            .iter()
            .filter(|t| {
                t.kind == TagKind::Def
                    && (t.name == "greet"
                        || t.name == "new"
                        || t.name == "main")
            })
            .collect();

        assert!(
            !method_defs.is_empty(),
            "Expected method/function defs (greet, new, main) in fixture — none found."
        );

        for def in &method_defs {
            assert_eq!(
                def.node_kind, "identifier",
                "Method/function def '{}' at line {} has node_kind='{}', expected 'identifier'. \
                 Wrong-anchor vintage drift detected — check .scm vintage.",
                def.name, def.line, def.node_kind
            );
        }
    }

    /// e05s02 gate: PageRank output is byte-stable for a fixed input.
    #[test]
    fn deterministic() {
        let tags_a = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let tags_b = extract_tags("src/b.rs", RUST_FIXTURE, Lang::Rust).unwrap();

        // Combine two "files" worth of tags so the graph has edges to rank.
        let mut all_tags = tags_a;
        all_tags.extend(tags_b);

        let run1 = rank_files(&all_tags);
        let run2 = rank_files(&all_tags);

        assert_eq!(
            run1.len(),
            run2.len(),
            "Ranked output length changed between runs"
        );
        for (a, b) in run1.iter().zip(run2.iter()) {
            assert_eq!(
                a.rel_fname, b.rel_fname,
                "File order changed between runs: {} vs {}",
                a.rel_fname, b.rel_fname
            );
            // Use bit-exact float comparison: same algorithm, same input → same IEEE 754 result.
            assert_eq!(
                a.score.to_bits(),
                b.score.to_bits(),
                "Score for '{}' changed between runs: {} vs {}",
                a.rel_fname,
                a.score,
                b.score
            );
        }
    }
}
