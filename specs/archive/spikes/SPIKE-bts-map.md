# SPIKE: bts-map grammar pinning + silent-empty-map validation

**Date:** 2026-06-17  
**Risk area:** e05 (bts-map Rust port)  
**Status:** COMPLETE — green  
**Spike location:** throwaway temp dir (not committed), reconstructable from this file.

---

## Question

Can aider's vendored `.scm` tag queries produce **non-empty** `def` AND `ref` captures
in Rust when pinned to matching grammar crates? What exact versions work? Does a grammar
mismatch fail silently (0 captures, no error), and is a per-language non-empty assertion
a mandatory CI gate?

---

## .scm capture-name convention (confirmed)

All three files use the same prefix scheme. Aider's `repomap.py` routes captures by prefix:

| Prefix | Meaning |
|---|---|
| `name.definition.*` | a definition identifier (what goes in the "defs" bucket) |
| `name.reference.*` | a reference/call site identifier ("refs" bucket) |
| `definition.*` | the surrounding node (not used for naming, used for ranking) |
| `reference.*` | the surrounding node (not used for naming, used for ranking) |

**Rust** (`tree-sitter-language-pack/rust-tags.scm`):
- Defs: `name.definition.class`, `.method`, `.function`, `.interface`, `.module`, `.macro`
- Refs: `name.reference.call`, `name.reference.implementation`

**Python** (`tree-sitter-language-pack/python-tags.scm`):
- Defs: `name.definition.constant`, `.class`, `.function`
- Refs: `name.reference.call`

**TypeScript** (`tree-sitter-languages/typescript-tags.scm`):
- Defs: `name.definition.function`, `.method`, `.class`, `.module`, `.interface`, `.type`, `.enum`
- Refs: `name.reference.type`, `name.reference.class`

Note: TypeScript exists ONLY in `tree-sitter-languages/`, not in `tree-sitter-language-pack/`.
Swift exists only in `tree-sitter-language-pack/`. The 10-language target spans both dirs.

---

## Grammar version selection

### Source of truth

`tree-sitter-language-pack v0.13.0` → `sources/language_definitions.json` pinned commits:

| Language | Pinned git rev | Commit date |
|---|---|---|
| Rust | `261b20226c04ef601adbdf185a800512a5f66291` | 2025-10-06 |
| Python | `26855eabccb19c6abf499fbc5b8dc7cc9ab8bc64` | 2025-09-15 |
| TypeScript | `75b3874edb2dc714fb1fd77a32013d0f8699989f` | 2025-01-30 |

### Crates.io nearest matches

No crates.io entry exactly matches the pinned commits. Nearest versions by publish date:

| Crate | Chosen version | Published | Exact match? |
|---|---|---|---|
| `tree-sitter-rust` | `0.23.2` | 2024-11-24 | No — closest prior to the Oct 2025 commit; 0.24.x (Apr 2025) is also close but pulls tree-sitter ^0.25 |
| `tree-sitter-python` | `0.23.6` | 2024-12-22 | No — closest prior to the Sep 2025 commit; 0.25.0 (Sep 2025) pulls tree-sitter ^0.25.8 which conflicts with 0.24.x core |
| `tree-sitter-typescript` | `0.23.2` | 2024-11-11 | No — closest prior to the Jan 2025 TS commit |
| `tree-sitter` (core) | `0.24.6` | 2024-12-27 | Required by all three grammar crates via `^0.24` |

**Version selection strategy:** Grammar crates with `^0.24` core requirements were chosen
to keep all three on the same core crate. Python 0.25.0 was skipped because it requires
`tree-sitter ^0.25.8`, which would force the other grammar crates to upgrade inconsistently.

---

## Working Cargo.toml `[dependencies]` block

```toml
[dependencies]
tree-sitter          = "=0.24.6"
tree-sitter-rust     = "=0.23.2"
tree-sitter-python   = "=0.23.6"
tree-sitter-typescript = "=0.23.2"
streaming-iterator   = "0.1"
```

Resolved (from Cargo.lock):

```
tree-sitter            0.24.6
tree-sitter-language   0.1.7      (re-exported by grammar crates)
tree-sitter-rust       0.23.2
tree-sitter-python     0.23.6
tree-sitter-typescript 0.23.2
streaming-iterator     0.1.9
```

---

## Full harness `src/main.rs`

```rust
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

const RUST_SCM: &str = include_str!("../queries/rust-tags.scm");
const PYTHON_SCM: &str = include_str!("../queries/python-tags.scm");
const TYPESCRIPT_SCM: &str = include_str!("../queries/typescript-tags.scm");

const RUST_SRC: &str = r#"
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

const PYTHON_SRC: &str = r#"
class Greeter:
    def greet(self, name):
        return f"Hello, {name}!"

def main():
    g = Greeter()
    result = g.greet("world")
    print(result)
"#;

const TYPESCRIPT_SRC: &str = r#"
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
"#;

fn count_captures(
    language: Language,
    scm: &str,
    source: &str,
    lang_name: &str,
) -> (usize, usize, bool) {
    let mut parser = Parser::new();
    parser.set_language(&language).expect("set_language failed");
    let tree = parser.parse(source, None).expect("parse failed");
    let root = tree.root_node();

    let query = match Query::new(&language, scm) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("[{}] Query compile ERROR: {:?}", lang_name, e);
            return (0, 0, false);
        }
    };

    let mut cursor = QueryCursor::new();
    // tree-sitter 0.24.x: QueryMatches implements StreamingIterator, NOT std::Iterator
    let mut matches = cursor.matches(&query, root, source.as_bytes());

    let mut defs = 0usize;
    let mut refs = 0usize;

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_name = &query.capture_names()[cap.index as usize];
            if cap_name.starts_with("name.definition.") {
                defs += 1;
            } else if cap_name.starts_with("name.reference.") {
                refs += 1;
            }
        }
    }

    (defs, refs, true)
}

fn run_lang(label: &str, language: Language, scm: &str, source: &str) -> bool {
    let (defs, refs, compiled) = count_captures(language, scm, source, label);
    let ok = compiled && defs > 0 && refs > 0;
    println!(
        "{}: defs={} refs={}  query_compiled={}  PASS={}",
        label, defs, refs, compiled, ok
    );
    ok
}

fn main() {
    println!("=== bts-map spike: grammar pinning verification ===\n");

    // tree-sitter 0.24.x API: grammar crates export LANGUAGE (LanguageFn constant)
    // NOT language() function (that was the 0.22.x and earlier API).
    // tree-sitter-typescript is special: LANGUAGE_TYPESCRIPT and LANGUAGE_TSX constants.
    let rust_lang: Language = tree_sitter_rust::LANGUAGE.into();
    let python_lang: Language = tree_sitter_python::LANGUAGE.into();
    let ts_lang: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();

    println!("--- Happy path (matched grammar + .scm, tree-sitter=0.24.6) ---");
    let rust_ok = run_lang("rust   [tree-sitter-rust=0.23.2]   ", rust_lang.clone(), RUST_SCM, RUST_SRC);
    let python_ok = run_lang("python [tree-sitter-python=0.23.6]", python_lang.clone(), PYTHON_SCM, PYTHON_SRC);
    let ts_ok = run_lang("ts     [tree-sitter-typescript=0.23.2]", ts_lang.clone(), TYPESCRIPT_SCM, TYPESCRIPT_SRC);

    println!();
    if rust_ok && python_ok && ts_ok {
        println!("ALL PASS: grammar pinning produces non-empty def+ref captures.");
    } else {
        println!("FAILURES DETECTED (see above).");
    }

    // Silent failure demo: a query that is structurally valid and compiles fine
    // but returns 0 defs because the fixture source doesn't use the queried construct.
    // This mimics a grammar version bump that changes WHICH tree shape a construct uses
    // without renaming any node type (which would cause a NodeType compile error in 0.24.x).
    let silent_query = r#"
; Structurally valid: module can contain function_item in Rust grammar
; but our fixture has no mod_item -> 0 defs, compiles fine, no error
(mod_item
    body: (declaration_list
        (function_item
            name: (identifier) @name.definition.method) @definition.method))

(call_expression
    function: (identifier) @name.reference.call) @reference.call
"#;

    println!("\n=== Failure-mode: SILENT empty-map case ===");
    let rust_lang2: Language = tree_sitter_rust::LANGUAGE.into();
    let (defs_s, refs_s, compiled_s) = count_captures(rust_lang2, silent_query, RUST_SRC, "rust-silent");
    println!("rust-silent: defs={} refs={}  query_compiled={}", defs_s, refs_s, compiled_s);
    if compiled_s && defs_s == 0 && refs_s > 0 {
        println!("CONFIRMED: compiled=true, 0 defs, non-zero refs. cargo build PASSES. Map is silently empty.");
    }
}
```

---

## Observed output

```
=== bts-map spike: grammar pinning verification ===

--- Happy path (matched grammar + .scm, tree-sitter=0.24.6) ---
rust   [tree-sitter-rust=0.23.2]   : defs=7 refs=6  query_compiled=true  PASS=true
python [tree-sitter-python=0.23.6]: defs=3 refs=3  query_compiled=true  PASS=true
ts     [tree-sitter-typescript=0.23.2]: defs=6 refs=1  query_compiled=true  PASS=true

ALL PASS: grammar pinning produces non-empty def+ref captures.

=== Failure-mode: SILENT empty-map case ===
rust-silent: defs=0 refs=1  query_compiled=true
CONFIRMED: compiled=true, 0 defs, non-zero refs. cargo build PASSES. Map is silently empty.
```

---

## Failure-mode analysis

### What tree-sitter 0.24.x validates at `Query::new()` time

| Mismatch type | Behavior in 0.24.x | Silent? |
|---|---|---|
| Unknown node type (e.g. `functiOn_definitioN`) | `QueryError { kind: NodeType }` — compile error | No |
| Unknown field name (e.g. `wrong_field:`) | `QueryError { kind: Field }` — compile error | No |
| Impossible parent-child structure | `QueryError { kind: Structure }` — compile error | No |
| Valid node types + valid fields + valid structure BUT wrong tree shape for this source | Compiles, 0 captures | **YES — SILENT** |

**Important finding:** tree-sitter 0.24.x is significantly stricter at query compile time than
older versions (pre-0.22). The ADR's "silent failure" scenario mostly becomes a compile-time
error in 0.24.x — BUT the STRUCTURAL mismatch remains silent.

### The confirmed real silent failure case

The `rust-silent` query uses `mod_item → declaration_list → function_item`, which is all
valid Rust grammar structure. The query compiles without error. But the fixture source has
no `mod_item`, so 0 defs are captured, while refs (call expressions) are still found.
`cargo build` passes. No test error. The map is silently empty for definitions.

**This is the production scenario:** a grammar bump changes HOW a construct is nested
(e.g., methods move from being direct children of `impl_item` to being inside a
`declaration_list` wrapper). The query still uses valid node type names, so it compiles.
But the tree shape no longer matches, so it returns 0.

### The aider dual-vintage structural drift (confirmed real)

The two `.scm` files for Rust differ in WHERE `@definition.method` attaches:

```scheme
; tree-sitter-language-pack vintage (the one we vendor):
(declaration_list
    (function_item
        name: (identifier) @name.definition.method) @definition.method)
;   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
;   @definition.method is captured on function_item

; tree-sitter-languages vintage (older):
(declaration_list
    (function_item
        name: (identifier) @name.definition.method)) @definition.method
;   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
;   @definition.method is captured on declaration_list (the CONTAINER)
```

Both compile. Both produce non-zero counts. But the older vintage anchors the definition
to the container node (`declaration_list`), not the method node. This means the "definition"
in the map spans the wrong byte range — every method in an `impl` block would map to the
same container. A non-empty-count assertion passes; only a node-type assertion would catch it.

---

## API changes: 0.22 → 0.24 (surprises for e05)

1. **Grammar crate API:** The `language()` function from 0.22.x is gone. Grammar crates
   now export `LANGUAGE: LanguageFn` (a constant). Calling `language()` gives `E0425`.
   Use `tree_sitter_rust::LANGUAGE.into()` to get a `Language`.

2. **TypeScript dual-grammar:** `tree-sitter-typescript` exports TWO constants:
   - `LANGUAGE_TYPESCRIPT` — for `.ts` files
   - `LANGUAGE_TSX` — for `.tsx` files
   There is no `language_typescript()` function. This means `bts map` needs filename
   extension logic to pick the right grammar (`.ts` → LANGUAGE_TYPESCRIPT, `.tsx` → LANGUAGE_TSX).

3. **`QueryMatches` implements `StreamingIterator`, not `std::Iterator`:** The standard
   `for m in matches` loop fails with `E0277`. Must add `streaming-iterator` as a dependency
   and use `while let Some(m) = matches.next()` with `use streaming_iterator::StreamingIterator`.
   This is a compile error, not a silent failure — but it's a gotcha that will hit e05 on day 1.

4. **`tree-sitter-language` crate:** Grammar crates re-export from `tree-sitter-language 0.1.7`,
   a thin ABI-stability shim. This is automatic and transparent; no explicit dep needed.

---

## VERDICT

### Is the grammar-pinning approach viable?

**YES.** Pinning `tree-sitter = "=0.24.6"` with `tree-sitter-rust = "=0.23.2"`,
`tree-sitter-python = "=0.23.6"`, and `tree-sitter-typescript = "=0.23.2"` produces
non-empty def+ref captures for all three languages using aider's vendored `.scm` files.

The versions are NOT an exact match to the `tree-sitter-language-pack v0.13.0` commit
timestamps (the pinned commits post-date the nearest crate releases by months), but the
`.scm` queries are stable enough that the nearest 0.23.x crates work correctly.

### What exact versions work?

```toml
tree-sitter          = "=0.24.6"
tree-sitter-rust     = "=0.23.2"
tree-sitter-python   = "=0.23.6"
tree-sitter-typescript = "=0.23.2"
streaming-iterator   = "0.1"
```

### Is the silent-empty-map real?

**YES, and nuanced.** In tree-sitter 0.24.x, many mismatch types that were silent in older
versions now produce compile-time errors (NodeType, Field, Structure validation). However,
the structural-mismatch case — where node types and fields are all valid but the tree shape
doesn't match — REMAINS SILENT. The `rust-silent` test confirms: `compiled=true`, `defs=0`,
`refs=1`, `cargo build` passes with no error.

### Is a per-language non-empty def+ref assertion a mandatory CI gate?

**YES, absolutely mandatory.** The structural-mismatch silent failure is real and survives
`cargo build`. The assertion `defs > 0 && refs > 0` is the minimum viable gate. A stronger
gate (`assert def node kind == "identifier"` or `assert def node range matches fixture source`)
would also catch the wrong-anchor silent corruption from vintage drift.

---

## Top things e05 must do differently from a naive port

1. **Use `LANGUAGE` constants, not `language()` functions.** All grammar crates in the
   0.24.x era export `LANGUAGE: LanguageFn`. Use `.into()` to convert to `Language`.
   TypeScript is special: use `LANGUAGE_TYPESCRIPT` for `.ts`, `LANGUAGE_TSX` for `.tsx`.

2. **Add `streaming-iterator` as an explicit dependency.** `QueryMatches` does not implement
   `std::Iterator` — it implements `StreamingIterator`. Without this dep and the trait import,
   `for m in matches` fails to compile. This is non-obvious and will block every developer
   hitting it for the first time.

3. **Per-language non-empty def+ref assertion in CI, not just `cargo build`.** The structural-
   mismatch silent failure is real in 0.24.x despite stronger compile-time validation. Each
   language fixture test must assert `defs > 0 AND refs > 0`. For correctness beyond just
   non-zero counts, also assert the captured identifier node kind and that def/ref names match
   the fixture source (catches wrong-anchor vintage drift from the `@definition.method` case).
