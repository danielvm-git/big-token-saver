#![deny(unsafe_code)]

mod error;
mod feedback;
mod graph;
mod render;
mod tags;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::feedback::FeedbackDb;
use crate::tags::{extract_tags, Lang, Tag};

/// bts-map — generate a ranked repo map from source tags.
///
/// Reads the current directory and emits a ranked list of source files,
/// ordered by PageRank over the def/ref symbol graph. Output is trimmed
/// to the requested token BUDGET (approximate; default 1024).
///
/// Supports regret-style learning: pass --feedback with a path to a
/// JSON feedback file, and PageRank scores are adjusted based on prior
/// selections (good files boosted, ignored files penalized).
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Token budget: maximum number of tokens in the ranked output (approximate).
    /// Files are included in PageRank order until the budget is exhausted.
    #[arg(short, long, default_value_t = 1024)]
    budget: usize,

    /// Path to a regret-learning feedback JSON file.
    /// When provided, PageRank scores are adjusted: files with positive
    /// feedback (--good) are boosted; files with negative feedback (--bad)
    /// are penalized. Missing or empty files are silently ignored.
    #[arg(short, long)]
    feedback: Option<PathBuf>,

    /// Root directory to scan (defaults to current directory).
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load regret-learning feedback if a path was provided.
    let feedback_db = cli
        .feedback
        .as_ref()
        .and_then(|p| FeedbackDb::load(p).ok());

    // Collect all source files under the target path.
    let mut all_tags: Vec<Tag> = Vec::new();
    collect_tags(&cli.path, &mut all_tags)?;

    if all_tags.is_empty() {
        eprintln!("bts-map: no source files found under {}", cli.path.display());
        return Ok(());
    }

    // Run PageRank (with regret adjustment if feedback is available).
    let ranked = crate::graph::rank_files(&all_tags, feedback_db.as_ref());

    // Render budget-bounded output.
    let output = crate::render::render(&ranked, &all_tags, cli.budget);
    println!("{}", output);

    Ok(())
}

/// Walk a directory recursively and extract tags from supported source files.
///
/// Skips common build/output directories (`.git`, `target`, `node_modules`,
/// `vendor`, `.direnv`, `dist`, `build`, `__pycache__`, `.tox`) and hidden
/// directories (those starting with `.` except the root itself) to avoid
/// scanning generated code and build artifacts.
fn collect_tags(root: &PathBuf, tags: &mut Vec<Tag>) -> Result<()> {
    /// Common build/output directory names to skip.
    const EXCLUDED_DIRS: &[&str] = &[
        ".git",
        "target",
        "node_modules",
        "vendor",
        ".direnv",
        "dist",
        "build",
        "__pycache__",
        ".tox",
    ];

    fn is_excluded(entry: &walkdir::DirEntry) -> bool {
        entry.file_type().is_dir()
            && entry
                .file_name()
                .to_str()
                .map(|name| {
                    // Skip hidden directories and common build/output names.
                    // Depth guard: never exclude the walk root itself (depth 0),
                    // otherwise `.` as root path would self-exclude.
                    (entry.depth() > 0 && name.starts_with('.')) || EXCLUDED_DIRS.contains(&name)
                })
                .unwrap_or(false)
    }

    let mut skipped_errors = 0u64;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_excluded(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                if let Some(path) = err.path() {
                    eprintln!("bts-map: skipping unreadable: {}: {}", path.display(), err);
                }
                skipped_errors += 1;
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lang = match Lang::from_ext(&path.to_string_lossy()) {
            Some(l) => l,
            None => continue,
        };
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("bts-map: warning: {}: {}", path.display(), err);
                skipped_errors += 1;
                continue;
            }
        };
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };
        match extract_tags(&rel, &source, lang) {
            Ok(file_tags) => tags.extend(file_tags),
            Err(e) => {
                eprintln!("bts-map: warning: {}: {}", rel, e);
                skipped_errors += 1;
            }
        }
    }
    if skipped_errors > 0 {
        eprintln!("bts-map: skipped {} unreadable/invalid file(s)", skipped_errors);
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::collect_tags;
    use crate::feedback::FeedbackDb;
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

        let run1 = rank_files(&all_tags, None);
        let run2 = rank_files(&all_tags, None);

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

    // ── e07s01: regret-learning feedback tests ────────────────────────────

    /// Files with positive feedback are boosted; negative feedback penalised.
    #[test]
    fn feedback_adjusts_scores() {
        let tags_a = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let tags_b = extract_tags("src/b.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let mut all_tags = tags_a;
        all_tags.extend(tags_b);

        // No feedback: same as baseline.
        let baseline = rank_files(&all_tags, None);

        // Good feedback on "src/a.rs" should boost it.
        let mut db = FeedbackDb::default();
        db.record("src/a.rs", true);
        db.record("src/a.rs", true);
        let boosted = rank_files(&all_tags, Some(&db));

        let a_baseline = baseline.iter().find(|r| r.rel_fname == "src/a.rs").unwrap();
        let a_boosted = boosted.iter().find(|r| r.rel_fname == "src/a.rs").unwrap();
        assert!(a_boosted.score > a_baseline.score,
            "Expected src/a.rs to be boosted (was {}, now {})",
            a_baseline.score, a_boosted.score);

        // Bad feedback on "src/a.rs" should penalise it.
        let mut db2 = FeedbackDb::default();
        db2.record("src/a.rs", false);
        db2.record("src/a.rs", false);
        let penalised = rank_files(&all_tags, Some(&db2));

        let a_penalised = penalised.iter().find(|r| r.rel_fname == "src/a.rs").unwrap();
        assert!(a_penalised.score < a_baseline.score,
            "Expected src/a.rs to be penalised (was {}, now {})",
            a_baseline.score, a_penalised.score);
    }

    /// Files with no feedback are unaffected (factor = 1.0).
    #[test]
    fn feedback_noop_for_unknown_files() {
        let tags = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let baseline = rank_files(&tags, None);
        let empty_db = FeedbackDb::default();
        let with_empty = rank_files(&tags, Some(&empty_db));

        for (b, w) in baseline.iter().zip(with_empty.iter()) {
            assert_eq!(b.rel_fname, w.rel_fname);
            assert!((b.score - w.score).abs() < 1e-10,
                "Empty feedback should not change scores: {} had {}, now {}",
                b.rel_fname, b.score, w.score);
        }
    }

    /// Feedback can invert the ranking when applied strongly enough.
    #[test]
    fn feedback_can_reorder() {
        let tags_a = extract_tags("src/a.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let tags_b = extract_tags("src/b.rs", RUST_FIXTURE, Lang::Rust).unwrap();
        let mut all_tags = tags_a;
        all_tags.extend(tags_b);

        let baseline = rank_files(&all_tags, None);
        let first_baseline = &baseline[0].rel_fname;

        // Heavily penalise the top-ranked file and boost the other.
        let other = if first_baseline == "src/a.rs" { "src/b.rs" } else { "src/a.rs" };

        let mut db = FeedbackDb::default();
        for _ in 0..10 {
            db.record(first_baseline, false); // bad
            db.record(other, true);            // good
        }
        let reordered = rank_files(&all_tags, Some(&db));

        // The reordered top should be the one we boosted.
        assert_eq!(
            reordered[0].rel_fname, other,
            "Expected {} to be first after feedback, but got {}",
            other, reordered[0].rel_fname
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

        let ranked = rank_files(&all_tags, None);

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

        let ranked = rank_files(&all_tags, None);
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

    // ── Integration tests for the directory-walk loop ──────────────────

    /// Walking from `.` (the default path) must not self-exclude.
    #[test]
    fn walk_from_dot_produces_tags() {
        let tmp = std::env::temp_dir().join("bts-map-test-walk-dot");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("main.rs"), "fn main() {}").unwrap();

        let mut tags = Vec::new();
        collect_tags(&tmp.join("."), &mut tags).unwrap();

        assert!(!tags.is_empty(), "walking '.' should find tags");
        assert!(
            tags.iter().any(|t| t.name == "main"),
            "should find 'main' in main.rs"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Walking a hidden-named root (e.g., `.hidden-project/`) should still
    /// scan its contents — only hidden *children* are excluded.
    #[test]
    fn walk_from_hidden_root_produces_tags() {
        let tmp = std::env::temp_dir().join(".bts-map-test-hidden-root");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("main.rs"), "fn main() {}").unwrap();

        let mut tags = Vec::new();
        collect_tags(&tmp, &mut tags).unwrap();

        assert!(
            !tags.is_empty(),
            "walking a hidden-named root should still find tags"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Hidden child directories (depth ≥ 1) must remain excluded.
    #[test]
    fn hidden_children_are_excluded() {
        let tmp = std::env::temp_dir().join("bts-map-test-hidden-children");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::create_dir_all(tmp.join(".hidden")).unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(
            tmp.join(".hidden/should_not_scan.rs"),
            "fn secret() {}",
        )
        .unwrap();

        let mut tags = Vec::new();
        collect_tags(&tmp, &mut tags).unwrap();

        // Should find main but NOT secret.
        assert!(
            tags.iter().any(|t| t.name == "main"),
            "should find main in src/main.rs"
        );
        assert!(
            !tags.iter().any(|t| t.name == "secret"),
            ".hidden/should_not_scan.rs should be excluded"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
