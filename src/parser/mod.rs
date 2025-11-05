//! Parser module for converting source code into an abstract syntax tree (AST).

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use crate::error::{Error, Result};
use std::path::Path;
use tree_sitter::Parser as TSParser;

/// The main parser struct that handles parsing source code.
#[allow(dead_code)]
#[allow(clippy::empty_line_after_outer_attr)]

pub struct Parser {
    /// The tree-sitter parser instance.
    parser: TSParser,
}

impl Parser {
    /// Creates a new parser for Python.

    pub fn new() -> Result<Self> {

        let mut parser = TSParser::new();

        // Get the Python language definition
        let language = tree_sitter_python::language();

        parser
            .set_language(language)
            .map_err(|e| Error::parser_error(format!("Failed to load language: {}", e)))?;

        Ok(Self { parser })
    }

    /// Parses a source file into a syntax tree.

    pub fn parse_file(&mut self, path: &Path) -> Result<tree_sitter::Tree> {

        let source_code = std::fs::read_to_string(path)
            .map_err(|e| Error::parser_error(format!("Failed to read file: {}", e)))?;

        self.parse_string(&source_code)
    }

    /// Parses a source code string into a syntax tree.

    pub fn parse_string(&mut self, source: &str) -> Result<tree_sitter::Tree> {

        self.parser
            .parse(source, None)
            .ok_or_else(|| Error::parser_error("Failed to parse source code".to_string()))
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_parser_initialization() {

        assert!(Parser::new().is_ok());
    }

    #[test]

    fn test_parse_string() {

        let mut parser = Parser::new().unwrap();

        let source = r#"
        def hello():
            print("Hello, world!")
        "#;

        let tree = parser.parse_string(source);

        assert!(tree.is_ok());
    }
}
