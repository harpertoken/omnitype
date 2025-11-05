//! Module for automatically fixing type-related issues in source code.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::{fs, path::Path};

use crate::{error::Result, types::TypeEnv, utils::find_python_files};

/// The main fixer that applies type fixes to source code.
#[allow(dead_code)]
#[allow(clippy::empty_line_after_outer_attr)]

pub struct Fixer {
    /// Type environment containing inferred types
    type_env: TypeEnv,

    /// Whether to apply changes in-place
    in_place: bool,
}

impl Fixer {
    /// Creates a new fixer with the given type environment.

    pub fn new(type_env: TypeEnv, in_place: bool) -> Self {
        Self { type_env, in_place }
    }

    /// Fixes type annotations in the specified file or directory.

    pub fn fix_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        if path.is_file() {
            self.fix_file(path)?;
        } else if path.is_dir() {
            for file in find_python_files(path) {
                self.fix_file(file)?;
            }
        }

        Ok(())
    }

    /// Fixes type annotations in a single source file.

    fn fix_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        if path.extension().and_then(|e| e.to_str()) != Some("py") {
            return Ok(());
        }

        let original = fs::read_to_string(path)?;

        let fixed = Self::fix_source(&original);

        if fixed != original && self.in_place {
            fs::write(path, fixed)?;
        }

        Ok(())
    }

    /// Generates a type annotation for a node.
    #[allow(dead_code)]

    fn generate_annotation(&self, node: &tree_sitter::Node, source: &[u8]) -> Option<String> {
        // For now, use simple heuristics based on variable names and context
        if let Ok(name) = node.utf8_text(source) {
            match name {
                "x" | "y" | "i" | "j" | "k" => Some("int".to_string()),
                "name" | "text" | "msg" => Some("str".to_string()),
                "flag" | "enabled" => Some("bool".to_string()),
                "items" | "data" => Some("List[Any]".to_string()),
                _ => Some("Any".to_string()),
            }
        } else {
            Some("Any".to_string())
        }
    }

    /// Text-based fixer to add Any annotations.

    fn fix_source(source: &str) -> String {
        let mut used_any = false;

        let mut out = String::with_capacity(source.len() + 64);

        for line in source.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("def ") {
                if let Some(paren_start) = trimmed.find('(') {
                    if let Some(paren_end) = trimmed[paren_start..].find(')') {
                        let indent_len = line.len() - trimmed.len();

                        let before = &trimmed[..paren_start + 1];

                        let params = &trimmed[paren_start + 1..paren_start + paren_end];

                        let after_params = &trimmed[paren_start + paren_end + 1..];

                        let fixed_params = Self::annotate_params(params, &mut used_any);

                        let mut signature_tail = after_params.to_string();

                        if !after_params.contains("->") {
                            if let Some(colon_idx) = signature_tail.find(':') {
                                let mut new_tail = String::new();

                                new_tail.push_str(" -> Any");

                                new_tail.push_str(&signature_tail[colon_idx..]);

                                signature_tail = new_tail;

                                used_any = true;
                            }
                        }

                        let mut rebuilt = String::new();

                        rebuilt.push_str(&" ".repeat(indent_len));

                        rebuilt.push_str(before);

                        rebuilt.push_str(&fixed_params);

                        rebuilt.push(')');

                        rebuilt.push_str(&signature_tail);

                        out.push_str(&rebuilt);

                        out.push('\n');

                        continue;
                    }
                }
            }

            out.push_str(line);

            out.push('\n');
        }

        if used_any
            && !source.contains("from typing import Any")
            && !out.contains("from typing import Any")
        {
            let lines: Vec<&str> = out.lines().collect();

            let mut insert_at = 0usize;

            if !lines.is_empty() && (lines[0].starts_with("#!") || lines[0].contains("coding")) {
                insert_at = 1;
            }

            let mut new_out = String::new();

            for (i, l) in lines.iter().enumerate() {
                if i == insert_at {
                    new_out.push_str("from typing import Any\n");
                }

                new_out.push_str(l);

                new_out.push('\n');
            }

            return new_out;
        }

        out
    }

    fn annotate_params(params: &str, _used_any: &mut bool) -> String {
        let mut parts = Vec::new();

        for raw in params.split(',') {
            let p = raw.to_string();

            let trimmed = p.trim();

            if trimmed.is_empty() {
                parts.push(p);

                continue;
            }

            let leading_idx = p.find(trimmed).unwrap_or(0);

            let leading = &p[..leading_idx];

            let trailing = &p[leading_idx + trimmed.len()..];

            if trimmed.starts_with('*') || trimmed.contains(':') {
                parts.push(p);

                continue;
            }

            let mut name = trimmed;

            let mut default = "";

            if let Some(eq_idx) = trimmed.find('=') {
                name = trimmed[..eq_idx].trim();

                default = &trimmed[eq_idx..];
            }

            let fixed = format!("{}{}: Any{}{}", leading, name, default, trailing);

            parts.push(fixed);
        }

        parts.join(",")
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_fixer_initialization() {
        let type_env = TypeEnv::new();

        let fixer = Fixer::new(type_env, false);

        assert!(!fixer.in_place);
    }
}
