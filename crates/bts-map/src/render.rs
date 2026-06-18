//! Skeleton renderer: produce a deterministic, budget-bounded text map.
//!
//! Inspired by aider's grep_ast.TreeContext but intentionally simpler:
//! - For each ranked file, print `path/to/file.rs` then its def tag lines (signatures).
//! - Consecutive-line gaps are elided with a `...` marker.
//! - Output is deterministic for fixed input: files sorted by rank desc, then fname asc;
//!   lines within a file sorted numerically; tie-breaking is stable.
//!
//! Token budget is enforced greedily: accumulate ranked output until the next file would
//! push the token count over the budget, then stop.  Token counting uses tiktoken-rs cl100k.
// Used by tests in main.rs; main() itself doesn't call these yet.
#![allow(dead_code)]

use std::collections::HashMap;

use tiktoken_rs::cl100k_base;

use crate::graph::RankedFile;
use crate::tags::{Tag, TagKind};

/// Render a budget-bounded skeleton map.
///
/// * `ranked` — files in descending PageRank order (from `rank_files`).
/// * `all_tags` — flat tag list (same tags passed to `rank_files`).
/// * `budget` — maximum token count for the rendered output (approximate).
///
/// Returns the rendered string (deterministic for fixed inputs).
pub fn render(ranked: &[RankedFile], all_tags: &[Tag], budget: usize) -> String {
    let bpe = cl100k_base().expect("cl100k_base tokenizer should always load");

    // Group def tags by file → sorted line numbers.
    let mut file_def_lines: HashMap<&str, Vec<(usize, &str)>> = HashMap::new();
    for tag in all_tags {
        if tag.kind == TagKind::Def {
            file_def_lines
                .entry(&tag.rel_fname)
                .or_default()
                .push((tag.line, &tag.name));
        }
    }
    // Sort each file's def lines numerically, then by name for tie-break.
    for lines in file_def_lines.values_mut() {
        lines.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        lines.dedup_by_key(|l| l.0); // deduplicate same-line entries
    }

    let mut out = String::new();
    let mut tokens_used = 0usize;

    for rf in ranked {
        let fname = rf.rel_fname.as_str();
        let lines = match file_def_lines.get(fname) {
            Some(l) if !l.is_empty() => l,
            _ => continue,
        };

        // Build the block for this file.
        let mut block = String::new();
        block.push_str(fname);
        block.push('\n');

        let mut prev_line: Option<usize> = None;
        for (line_no, name) in lines {
            if let Some(prev) = prev_line {
                if *line_no > prev + 1 {
                    block.push_str("    ...\n");
                }
            }
            block.push_str(&format!("    {}: {}\n", line_no + 1, name));
            prev_line = Some(*line_no);
        }

        // Token-budget check: count tokens for this block.
        let block_tokens = bpe.encode_ordinary(&block).len();
        if tokens_used + block_tokens > budget && tokens_used > 0 {
            // Budget exhausted; stop adding files.
            break;
        }
        out.push_str(&block);
        tokens_used += block_tokens;
    }

    out
}

/// Count tokens in a string using the cl100k_base tokenizer.
pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().expect("cl100k_base tokenizer should always load");
    bpe.encode_ordinary(text).len()
}
