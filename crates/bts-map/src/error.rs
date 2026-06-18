//! Typed error enum for bts-map library code (thiserror at boundaries).
//! Items are public API used in tests; will be called from main in e05s03.
#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MapError {
    #[error("Query compile failed: {0}")]
    QueryCompile(String),

    #[error("Parse failed for file: {0}")]
    ParseFailed(String),
}
