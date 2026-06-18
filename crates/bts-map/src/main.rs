#![deny(unsafe_code)]

mod error;
mod graph;
mod render;
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

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::graph::rank_files;
    use crate::render::{count_tokens, render};
    use crate::tags::{extract_tags, Lang, TagKind};

    // ── Existing e05s02 gates ──────────────────────────────────────────────

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

        let method_defs: Vec<_> = tags
            .iter()
            .filter(|t| {
                t.kind == TagKind::Def
                    && (t.name == "greet" || t.name == "new" || t.name == "main")
            })
            .collect();

        assert!(
            !method_defs.is_empty(),
            "Expected method/function defs (greet, new, main) in fixture — none found."
        );

        for def in &method_defs {
            assert_eq!(
                def.node_kind, "identifier",
                "Method/function def '{}' at line {} has node_kind='{}', expected 'identifier'.",
                def.name, def.line, def.node_kind
            );
        }
    }

    /// e05s02 gate: PageRank output is byte-stable for a fixed input.
    #[test]
    fn deterministic() {
        let tags_a = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let tags_b = extract_tags("src/b.rs", RUST_FIXTURE, Lang::Rust).unwrap();

        let mut all_tags = tags_a;
        all_tags.extend(tags_b);

        let run1 = rank_files(&all_tags);
        let run2 = rank_files(&all_tags);

        assert_eq!(run1.len(), run2.len(), "Ranked output length changed between runs");
        for (a, b) in run1.iter().zip(run2.iter()) {
            assert_eq!(
                a.rel_fname, b.rel_fname,
                "File order changed between runs: {} vs {}",
                a.rel_fname, b.rel_fname
            );
            assert_eq!(
                a.score.to_bits(),
                b.score.to_bits(),
                "Score for '{}' changed between runs: {} vs {}",
                a.rel_fname, a.score, b.score
            );
        }
    }

    // ── e05s03: per-language non-empty def+ref gates (mandatory d10) ───────
    //
    // Each test asserts defs>0 AND refs>0 for its language. These tests catch
    // silent grammar/.scm mismatch that compile cleanly but produce 0 captures.

    fn assert_defs_and_refs(rel_fname: &str, source: &str, lang: Lang) {
        let tags = extract_tags(rel_fname, source, lang)
            .unwrap_or_else(|e| panic!("{} extract_tags failed: {}", lang.name(), e));
        let defs = tags.iter().filter(|t| t.kind == TagKind::Def).count();
        let refs = tags.iter().filter(|t| t.kind == TagKind::Ref).count();
        assert!(
            defs > 0,
            "{}: expected >0 defs, got 0. Check .scm or grammar pin.",
            lang.name()
        );
        assert!(
            refs > 0,
            "{}: expected >0 refs, got 0. Check .scm or grammar pin.",
            lang.name()
        );
    }

    #[test]
    fn python_tags() {
        assert_defs_and_refs(
            "src/greeter.py",
            r#"
class Greeter:
    def greet(self, name):
        return f"Hello, {name}!"

def main():
    g = Greeter()
    result = g.greet("world")
    print(result)
"#,
            Lang::Python,
        );
    }

    #[test]
    fn typescript_tags() {
        assert_defs_and_refs(
            "src/greeter.ts",
            r#"
interface Greeter {
    greet(name: string): string;
}

class SimpleGreeter implements Greeter {
    greet(name: string): string {
        return `Hello, ${name}!`;
    }
}

function main(): void {
    const g = new SimpleGreeter();
    const msg = g.greet("world");
    console.log(msg);
}
"#,
            Lang::TypeScript,
        );
    }

    #[test]
    fn javascript_tags() {
        assert_defs_and_refs(
            "src/greeter.js",
            r#"
class Greeter {
    greet(name) {
        return `Hello, ${name}!`;
    }
}

function main() {
    const g = new Greeter();
    console.log(g.greet("world"));
    main();
}
"#,
            Lang::JavaScript,
        );
    }

    #[test]
    fn go_tags() {
        assert_defs_and_refs(
            "src/greeter.go",
            r#"
package main

import "fmt"

type Greeter struct{}

func (g Greeter) Greet(name string) string {
    return fmt.Sprintf("Hello, %s!", name)
}

func main() {
    g := Greeter{}
    fmt.Println(g.Greet("world"))
}
"#,
            Lang::Go,
        );
    }

    #[test]
    fn java_tags() {
        assert_defs_and_refs(
            "src/Greeter.java",
            r#"
public class Greeter {
    public String greet(String name) {
        return "Hello, " + name + "!";
    }

    public static void main(String[] args) {
        Greeter g = new Greeter();
        System.out.println(g.greet("world"));
    }
}
"#,
            Lang::Java,
        );
    }

    #[test]
    fn ruby_tags() {
        assert_defs_and_refs(
            "src/greeter.rb",
            r#"
class Greeter
  def greet(name)
    "Hello, #{name}!"
  end
end

def main
  g = Greeter.new
  g.greet("world")
end
"#,
            Lang::Ruby,
        );
    }

    #[test]
    fn c_tags() {
        assert_defs_and_refs(
            "src/greeter.c",
            r#"
#include <stdio.h>

struct Point {
    int x;
    int y;
};

int add(int a, int b) {
    return a + b;
}

int main(void) {
    int result = add(1, 2);
    printf("result: %d\n", result);
    return 0;
}
"#,
            Lang::C,
        );
    }

    #[test]
    fn cpp_tags() {
        assert_defs_and_refs(
            "src/greeter.cpp",
            r#"
#include <cstdio>

int add(int a, int b) {
    return a + b;
}

class Greeter {
public:
    void greet() {}
};

int main() {
    int result = add(1, 2);
    printf("result: %d\n", result);
    return 0;
}
"#,
            Lang::Cpp,
        );
    }

    #[test]
    fn swift_tags() {
        assert_defs_and_refs(
            "src/Greeter.swift",
            r#"
class Greeter {
    func greet(name: String) -> String {
        return "Hello, \(name)!"
    }
}

func main() {
    let g = Greeter()
    print(g.greet(name: "world"))
}
"#,
            Lang::Swift,
        );
    }

    // ── e05s03: token budget test ──────────────────────────────────────────

    /// Budget test: rendered output must never exceed the requested budget.
    ///
    /// Uses two copies of the Rust fixture (different "files") so the graph has
    /// enough edges to rank, then checks multiple budget sizes.
    #[test]
    fn budget() {
        let tags_a = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let tags_b = extract_tags("src/b.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let mut all_tags = tags_a;
        all_tags.extend(tags_b);

        let ranked = rank_files(&all_tags);

        for &budget in &[50, 100, 200, 512, 1024] {
            let output = render(&ranked, &all_tags, budget);
            let actual_tokens = count_tokens(&output);
            assert!(
                actual_tokens <= budget,
                "budget={}: rendered output has {} tokens, exceeds budget",
                budget,
                actual_tokens
            );
        }
    }

    // ── e05s03: snapshot test ─────────────────────────────────────────────

    /// Snapshot test: map output for the fixture repo is stable.
    ///
    /// The fixture repo lives in tests/fixtures/ and contains files across
    /// several of the 10 supported languages. This test runs the map at a
    /// fixed budget and asserts the output matches the committed snapshot.
    ///
    /// To regenerate the snapshot (e.g. after intentional output changes):
    ///   GENERATE_SNAPSHOT=1 cargo test -p bts-map snapshot
    #[test]
    fn snapshot() {
        use std::path::Path;

        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

        // Collect tags from all fixture files.
        let mut all_tags = Vec::new();
        let files = [
            ("greeter.rs", Lang::Rust),
            ("greeter.py", Lang::Python),
            ("greeter.ts", Lang::TypeScript),
            ("greeter.go", Lang::Go),
            ("greeter.java", Lang::Java),
        ];

        for (fname, lang) in &files {
            let path = fixture_dir.join(fname);
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fname, e));
            let rel = format!("tests/fixtures/{}", fname);
            let tags = extract_tags(&rel, &source, *lang)
                .unwrap_or_else(|e| panic!("extract_tags failed for {}: {}", fname, e));
            all_tags.extend(tags);
        }

        let ranked = rank_files(&all_tags);
        let output = render(&ranked, &all_tags, 256);

        // Check for GENERATE_SNAPSHOT env var to allow explicit regeneration.
        // is_ok_and guards against accidental GENERATE_SNAPSHOT=0 triggering a write.
        let generate = std::env::var("GENERATE_SNAPSHOT").is_ok_and(|v| v == "1");

        let expected_path = fixture_dir.join("expected_snapshot.txt");

        if generate {
            std::fs::write(&expected_path, &output)
                .unwrap_or_else(|e| panic!("Failed to write snapshot {}: {}", expected_path.display(), e));
            eprintln!("Snapshot regenerated at {}", expected_path.display());
            return;
        }

        // ASSERT that the expected file exists and matches.
        assert!(
            expected_path.exists(),
            "Snapshot file not found at {}. If this is the first run, generate with:\n\
             GENERATE_SNAPSHOT=1 cargo test -p bts-map snapshot\n\
             Then review and commit the resulting file.\n\
             Actual output would be:\n{}\n",
            expected_path.display(),
            output
        );

        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("Failed to read snapshot {}: {}", expected_path.display(), e));

        assert_eq!(
            output, expected,
            "Snapshot mismatch! To regenerate during development:\n\
             GENERATE_SNAPSHOT=1 cargo test -p bts-map snapshot\n\
             Then review the diff and commit.\n\
             Actual:\n{}",
            output
        );
    }
}
