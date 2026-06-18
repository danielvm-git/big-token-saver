//! Def/ref graph and PageRank over extracted tags.
//!
//! Graph shape (aider repomap style):
//!   - Each file is a node.
//!   - Each `(file_with_ref, file_with_def)` pair adds a directed edge:
//!     file_with_ref → file_with_def  (the ref "points to" the definition).
//!   - Edge weight = number of shared identifier names between the two files.
//!
//! PageRank is hand-rolled power-iteration (petgraph ships none).
//! Determinism: fixed iteration count + alphabetic stable sort of node keys.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::feedback::FeedbackDb;
use crate::tags::{Tag, TagKind};

/// A ranked entry in the output.
#[derive(Debug, Clone)]
pub struct RankedFile {
    pub rel_fname: String,
    /// PageRank score (optionally adjusted by regret-learning feedback).
    pub score: f64,
    /// Unadjusted PageRank score (before feedback).
    pub raw_score: f64,
    /// Feedback adjustment factor (1.0 = no adjustment).
    pub feedback_factor: f64,
}

/// Build a directed file-dependency graph from a flat slice of tags, then run
/// PageRank and return files sorted by descending score (tie-break: file name
/// ascending for byte-stability on fixed input).
///
/// When `feedback_db` is provided, scores are multiplied by a regret-learning
/// factor: files with positive feedback are boosted; files with negative feedback
/// are penalised; files with no feedback are unchanged.
pub fn rank_files(tags: &[Tag], feedback_db: Option<&FeedbackDb>) -> Vec<RankedFile> {
    // Group tags by (file, name) → kind.
    let mut file_defs: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut file_refs: HashMap<&str, Vec<&str>> = HashMap::new();

    for tag in tags {
        match tag.kind {
            TagKind::Def => file_defs
                .entry(&tag.rel_fname)
                .or_default()
                .push(&tag.name),
            TagKind::Ref => file_refs
                .entry(&tag.rel_fname)
                .or_default()
                .push(&tag.name),
        }
    }

    // Collect all unique file names (sorted for deterministic node numbering).
    let mut files: Vec<&str> = {
        let set: HashSet<&str> = tags.iter().map(|t| t.rel_fname.as_str()).collect();
        let mut v: Vec<&str> = set.into_iter().collect();
        v.sort_unstable();
        v
    };
    files.sort_unstable();

    if files.is_empty() {
        return Vec::new();
    }

    // Map file name → NodeIndex.
    let mut graph: DiGraph<&str, f64> = DiGraph::new();
    let node_map: HashMap<&str, NodeIndex> = files
        .iter()
        .map(|&f| (f, graph.add_node(f)))
        .collect();

    // For each (ref_file, def_file) pair, accumulate edge weight.
    let mut edge_weights: HashMap<(NodeIndex, NodeIndex), f64> = HashMap::new();

    for (&ref_file, ref_names) in &file_refs {
        for (&def_file, def_names) in &file_defs {
            if ref_file == def_file {
                continue;
            }
            let shared = ref_names
                .iter()
                .filter(|&&rn| def_names.contains(&rn))
                .count() as f64;
            if shared > 0.0 {
                let from = node_map[ref_file];
                let to = node_map[def_file];
                *edge_weights.entry((from, to)).or_insert(0.0) += shared;
            }
        }
    }

    for ((from, to), weight) in edge_weights {
        graph.add_edge(from, to, weight);
    }

    let n = files.len();
    let scores = pagerank(&graph, n, 0.85, 50);

    let mut result: Vec<RankedFile> = files
        .iter()
        .enumerate()
        .map(|(i, &f)| {
            let raw = scores[i];
            let factor = feedback_db.map(|db| db.factor(f)).unwrap_or(1.0);
            RankedFile {
                rel_fname: f.to_string(),
                score: raw * factor,
                raw_score: raw,
                feedback_factor: factor,
            }
        })
        .collect();
    result.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.rel_fname.cmp(&b.rel_fname))
    });
    result
}

/// Hand-rolled power-iteration PageRank.
///
/// `damping` — typically 0.85
/// `iterations` — fixed count (deterministic; no convergence threshold)
///
/// Returns a score vector indexed by NodeIndex (matches insertion order of `files`).
fn pagerank(graph: &DiGraph<&str, f64>, n: usize, damping: f64, iterations: u32) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }

    let init = 1.0 / n as f64;
    let mut scores = vec![init; n];

    let out_weight_sums: Vec<f64> = (0..n)
        .map(|i| {
            graph
                .edges(NodeIndex::new(i))
                .map(|e| e.weight())
                .sum::<f64>()
        })
        .collect();

    for _ in 0..iterations {
        let mut new_scores = vec![(1.0 - damping) / n as f64; n];

        for i in 0..n {
            let out_sum = out_weight_sums[i];
            if out_sum <= 0.0 {
                // Dangling node: distribute score evenly (teleportation).
                let share = damping * scores[i] / n as f64;
                for slot in &mut new_scores {
                    *slot += share;
                }
                continue;
            }
            for edge in graph.edges(NodeIndex::new(i)) {
                let j = edge.target().index();
                new_scores[j] += damping * scores[i] * edge.weight() / out_sum;
            }
        }

        scores = new_scores;
    }

    scores
}
