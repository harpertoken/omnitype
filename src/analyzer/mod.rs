//! Static analysis for type inference and checking.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::{collections::HashMap, path::Path};
use tree_sitter::{Node, Tree};

use crate::{error::Result, parser::Parser, types::Type};

/// Simple diagnostic record produced by lightweight analysis.
#[derive(Debug, Clone, serde::Serialize)]

pub struct Diagnostic {
    /// File path of the diagnostic.
    pub path: String,
    /// 0-based line number where the issue was found.
    pub line: usize,
    /// 0-based column number where the issue was found.
    pub column: usize,
    /// Human-readable message.
    pub message: String,
    /// Severity string (e.g., "warning", "error").
    pub severity: String,
}

/// Per-file, lightweight analysis summary.
#[derive(Debug, Clone, serde::Serialize)]

pub struct AnalysisResult {
    /// Absolute or input path of the analyzed file.
    pub path: String,
    /// Number of top-level or nested Python function definitions.
    pub function_count: usize,
    /// Number of Python class definitions.
    pub class_count: usize,
    /// Collected diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// The main analyzer that performs static type checking and inference.
#[allow(dead_code)]
#[allow(clippy::empty_line_after_outer_attr)]

pub struct Analyzer {
    /// Type environment storing variable types in different scopes.
    type_env: HashMap<String, Type>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    /// Creates a new analyzer with an empty type environment.

    pub fn new() -> Self {
        Self { type_env: HashMap::new() }
    }

    /// Analyzes a syntax tree and infers types.

    pub fn analyze(&mut self, tree: &Tree, source: &[u8]) -> Result<()> {
        let root_node = tree.root_node();

        self.visit_node(&root_node, source)?;

        Ok(())
    }

    /// Visits a node in the syntax tree and processes it.

    fn visit_node(&mut self, node: &Node, source: &[u8]) -> Result<()> {
        let kind = node.kind();

        match kind {
            "function_definition" => {
                // Visit parameters and body
                if let Some(params) = node.child_by_field_name("parameters") {
                    self.visit_node(&params, source)?;
                }

                if let Some(body) = node.child_by_field_name("body") {
                    self.visit_node(&body, source)?;
                }
            },
            "class_definition" => {
                // Visit class body
                if let Some(body) = node.child_by_field_name("body") {
                    self.visit_node(&body, source)?;
                }
            },
            "assignment" => {
                // Infer type from right side
                if let Some(value) = node.child_by_field_name("right") {
                    let _ = self.infer_expression_type(&value, source);
                }
            },
            "call" => {
                // Infer types for function calls
                let _ = self.infer_expression_type(node, source);
            },
            _ => {
                // Recursively visit children
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    self.visit_node(&child, source)?;
                }
            },
        }

        Ok(())
    }

    /// Infers the type of an expression node.

    fn infer_expression_type(&self, node: &Node, source: &[u8]) -> Result<Type> {
        let kind = node.kind();

        match kind {
            "integer" => Ok(Type::Int),
            "string" => Ok(Type::Str),
            "true" | "false" => Ok(Type::Bool),
            "list" | "list_comprehension" => Ok(Type::List(Box::new(Type::Unknown))),
            "dictionary" => Ok(Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown))),
            "call" => {
                // For now, assume unknown return type
                Ok(Type::Unknown)
            },
            "identifier" => {
                // Look up in type environment
                let name = node.utf8_text(source)?;

                Ok(self.type_env.get(name).cloned().unwrap_or(Type::Unknown))
            },
            _ => Ok(Type::Unknown),
        }
    }
}

impl Analyzer {
    /// Performs a minimal analysis on a Python source file: counts functions
    /// and classes.

    pub fn analyze_python_file(path: &Path) -> Result<AnalysisResult> {
        let mut parser = Parser::new()?;

        let tree = parser.parse_file(path)?;

        let root = tree.root_node();

        let mut cursor = root.walk();

        let mut function_count = 0usize;

        let mut class_count = 0usize;

        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        // Simple DFS over all nodes
        let mut stack: Vec<Node> = Vec::new();

        for child in root.children(&mut cursor) {
            stack.push(child);
        }

        while let Some(node) = stack.pop() {
            let kind = node.kind();

            match kind {
                "function_definition" => {
                    function_count += 1;

                    // Check for missing parameter and return annotations
                    // parameters node is available via field name
                    if let Some(params) = node.child_by_field_name("parameters") {
                        // Iterate over parameters' children
                        let mut c = params.walk();

                        for p in params.children(&mut c) {
                            let p_kind = p.kind();

                            let is_typed = p_kind == "typed_parameter";

                            // If it's clearly a parameter and not typed, flag it
                            if !is_typed
                                && (p_kind == "identifier" || p_kind == "default_parameter")
                            {
                                let pos = p.start_position();

                                diagnostics.push(Diagnostic {
                                    path: path.to_string_lossy().to_string(),
                                    line: pos.row,
                                    column: pos.column,
                                    message: "Missing type annotation for parameter".to_string(),
                                    severity: "warning".to_string(),
                                });
                            }
                        }
                    }

                    // Return annotation: field name is often "return_type" in tree-sitter-python
                    if node.child_by_field_name("return_type").is_none() {
                        let pos = node.start_position();

                        diagnostics.push(Diagnostic {
                            path: path.to_string_lossy().to_string(),
                            line: pos.row,
                            column: pos.column,
                            message: "Missing return type annotation".to_string(),
                            severity: "warning".to_string(),
                        });
                    }
                },
                "class_definition" => class_count += 1,
                _ => {},
            }

            let mut child_cursor = node.walk();

            for child in node.children(&mut child_cursor) {
                stack.push(child);
            }
        }

        Ok(AnalysisResult {
            path: path.to_string_lossy().to_string(),
            function_count,
            class_count,
            diagnostics,
        })
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_analyzer_initialization() {
        let analyzer = Analyzer::new();

        assert!(analyzer.type_env.is_empty());
    }
}
