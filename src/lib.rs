//! omnitype: A hybrid type checker for Python and other dynamic languages.
//!
//! This library provides static and runtime type checking capabilities,
//! with support for type inference and automatic type annotation.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod analyzer;
pub mod error;
pub mod fixer;
pub mod parser;
pub mod solver;
pub mod tracer;
pub mod types;
pub mod utils;

/// Re-exports commonly used types and traits.

pub mod prelude {

    pub use crate::error::{Error, Result};
}

/// The main entry point for the omnitype application.

pub fn run() -> crate::error::Result<()> {

    // Application initialization and execution will be implemented here
    Ok(())
}
