//! Runtime type tracing for dynamic type information collection.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::NamedTempFile;
use wait_timeout::ChildExt;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};

use crate::{
    error::{Error, Result},
    types::Type,
};

/// Represents a runtime type trace.
#[derive(Debug, Default)]

pub struct TypeTrace {
    /// Map from variable names to their observed types
    pub variables: HashMap<String, Vec<Type>>,

    /// Map from function names to their argument and return types
    /// Format: (function_name, (argument_types_per_call,
    /// return_types_per_call))
    pub functions: HashMap<String, (Vec<Vec<Type>>, Vec<Type>)>,
}

impl TypeTrace {
    /// Add a variable observation to the trace

    pub fn add_variable(&mut self, name: String, type_info: Type) {
        self.variables.entry(name).or_default().push(type_info);
    }

    /// Add a function call observation to the trace

    pub fn add_function_call(&mut self, name: String, args: Vec<Type>, return_type: Type) {
        let entry = self.functions.entry(name).or_default();

        entry.0.push(args);

        entry.1.push(return_type);
    }

    /// Get unique types for a variable

    pub fn get_variable_types(&self, name: &str) -> Vec<&Type> {
        if let Some(types) = self.variables.get(name) {
            let mut seen = HashSet::new();

            let mut unique_types = Vec::new();

            for t in types {
                if seen.insert(t) {
                    unique_types.push(t);
                }
            }

            unique_types
        } else {
            Vec::new()
        }
    }
}

/// The main runtime tracer that collects type information.
#[allow(clippy::empty_line_after_outer_attr)]

pub struct RuntimeTracer {
    /// Accumulated type traces
    traces: TypeTrace,

    /// Whether to enable detailed logging
    verbose: bool,
}

impl RuntimeTracer {
    /// Creates a new runtime tracer.

    pub fn new(verbose: bool) -> Self {
        Self { traces: TypeTrace::default(), verbose }
    }

    /// Runs the tracer on the specified test file or module.

    pub fn run<P: AsRef<Path>>(&mut self, path: P, test_name: Option<&str>) -> Result<()> {
        let path = path.as_ref();

        if self.verbose {
            println!("Running runtime tracer on: {:?}", path);

            if let Some(name) = test_name {
                println!("Specific test: {}", name);
            }
        }

        // Check if the file exists and is a Python file
        if !path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {:?}", path),
            )));
        }

        if path.extension().and_then(|e| e.to_str()) != Some("py") {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File must be a Python file (.py)",
            )));
        }

        // Create a temporary instrumented version of the Python file
        let instrumented_content = if let Some(test_name) = test_name {
            self.create_specific_test_content(path, test_name)?
        } else {
            self.instrument_python_file(path)?
        };

        // Create a temporary file with proper cleanup handling
        let temp_file = NamedTempFile::with_suffix(".py").map_err(|e| {
            Error::Io(std::io::Error::other(format!("Failed to create temp file: {}", e)))
        })?;

        // Write the instrumented content
        fs::write(temp_file.path(), instrumented_content)?;

        // Allow override of the Python interpreter and prevent hangs with a timeout
        let python = std::env::var("OMNITYPE_PYTHON")
            .or_else(|_| std::env::var("PYTHON"))
            .unwrap_or_else(|_| "python3".to_string());

        // Execute the instrumented Python file
        let mut child = Command::new(python)
            .arg(temp_file.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Error::Io(std::io::Error::other(format!("Failed to spawn Python: {}", e)))
            })?;

        // Wait up to 60 seconds for Python to finish
        let status = child
            .wait_timeout(std::time::Duration::from_secs(60))
            .map_err(|e| {
                Error::Io(std::io::Error::other(format!("Error waiting for Python: {}", e)))
            })?;

        let output = if let Some(status) = status {
            let mut out = Vec::new();

            let mut err = Vec::new();

            if let Some(mut stdout) = child.stdout.take() {
                use std::io::Read;

                stdout.read_to_end(&mut out).ok();
            }

            if let Some(mut stderr) = child.stderr.take() {
                use std::io::Read;

                stderr.read_to_end(&mut err).ok();
            }

            Ok(std::process::Output { status, stdout: out, stderr: err })
        } else {
            let _ = child.kill();

            Err(Error::Io(std::io::Error::other("Python execution timed out")))
        };

        // temp_file is automatically cleaned up when it goes out of scope

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    if self.verbose {
                        eprintln!("Python execution failed: {}", stderr);
                    }

                    return Err(Error::Io(std::io::Error::other(format!(
                        "Python execution failed: {}",
                        stderr
                    ))));
                }

                let stdout = String::from_utf8_lossy(&output.stdout);

                self.parse_trace_output(&stdout)?;

                if self.verbose {
                    println!("Trace collection completed successfully");

                    self.print_trace_summary();
                }
            },
            Err(e) => {
                return Err(Error::Io(std::io::Error::other(format!(
                    "Failed to execute Python: {}",
                    e
                ))));
            },
        }

        Ok(())
    }

    /// Instrument a Python file with tracing code

    fn instrument_python_file<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let content = fs::read_to_string(path)?;

        // Create a comprehensive tracing system using sys.settrace
        let tracer_code = r#"
import sys
import json
import types
import inspect
import functools

# Runtime type tracer with call tracing
class TypeTracer:
    def __init__(self):
        self.traces = {"variables": {}, "functions": {}}
        self.call_stack = []
        self.in_trace = False

    def get_type_name(self, value):
        if value is None:
            return "None"
        elif isinstance(value, bool):
            return "bool"
        elif isinstance(value, int):
            return "int"
        elif isinstance(value, float):
            return "float"
        elif isinstance(value, str):
            return "str"
        elif isinstance(value, bytes):
            return "bytes"
        elif isinstance(value, list):
            if value:
                inner_type = self.get_type_name(value[0])
                return f"List[{inner_type}]"
            return "List[Any]"
        elif isinstance(value, dict):
            if value:
                key_type = self.get_type_name(next(iter(value.keys())))
                val_type = self.get_type_name(next(iter(value.values())))
                return f"Dict[{key_type}, {val_type}]"
            return "Dict[Any, Any]"
        elif isinstance(value, tuple):
            if value:
                types_list = [self.get_type_name(item) for item in value]
                return f"Tuple[{', '.join(types_list)}]"
            return "Tuple[()]"
        elif isinstance(value, set):
            if value:
                inner_type = self.get_type_name(next(iter(value)))
                return f"Set[{inner_type}]"
            return "Set[Any]"
        else:
            return type(value).__name__

    def trace_function_call(self, func_name, args, result):
        arg_types = [self.get_type_name(arg) for arg in args]
        result_type = self.get_type_name(result)

        if func_name not in self.traces["functions"]:
            self.traces["functions"][func_name] = {"args": [], "returns": []}

        self.traces["functions"][func_name]["args"].append(arg_types)
        self.traces["functions"][func_name]["returns"].append(result_type)

    def trace_calls(self, frame, event, arg):
        if self.in_trace:
            return self.trace_calls
        # Only trace the instrumented file
        if frame.f_code.co_filename != __file__:
            return self.trace_calls

        self.in_trace = True
        try:
            if event == 'call':
                func_name = frame.f_code.co_name
                if not func_name.startswith('_') and func_name not in ['<module>', 'trace_calls']:
                    # Get function arguments
                    args = []
                    arg_names = frame.f_code.co_varnames[:frame.f_code.co_argcount]
                    for name in arg_names:
                        if name in frame.f_locals and name != 'self':
                            args.append(frame.f_locals[name])

                    self.call_stack.append((func_name, args))

            elif event == 'return':
                if self.call_stack:
                    func_name, args = self.call_stack.pop()
                    if not func_name.startswith('_'):
                        self.trace_function_call(func_name, args, arg)
        finally:
            self.in_trace = False

        return self.trace_calls
    def print_traces(self):
        print("TRACE_OUTPUT_START")
        print(json.dumps(self.traces, indent=2))
        print("TRACE_OUTPUT_END")

_tracer = TypeTracer()

"#
        .to_string();

        // Append the original code directly
        let mut full_code = tracer_code;

        full_code.push_str(&content);

        full_code.push_str(
            r#"

# Set up call tracing
sys.settrace(_tracer.trace_calls)

# Only run test functions - safer approach
current_module = sys.modules[__name__]

# Run test functions only (following test_* convention)
for name in dir(current_module):
    obj = getattr(current_module, name)
    if callable(obj) and name.startswith('test_') and not name.startswith('_'):
        try:
            sig = inspect.signature(obj)
            has_required = any(
                p.default is inspect._empty
                and p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
                for p in sig.parameters.values()
            )
            if has_required:
                print(f"Skipping {name}: requires arguments")
                continue
            print(f"Running test: {name}")
            obj()
        except Exception as e:
            print(f"Error in test {name}: {e}")
# Note: Other functions will be traced when called by test functions
# This avoids the security risk of calling arbitrary functions with guessed arguments

# Disable tracing
sys.settrace(None)

_tracer.print_traces()
"#,
        );

        Ok(full_code)
    }

    /// Create instrumented content for a specific test function

    fn create_specific_test_content<P: AsRef<Path>>(
        &self,
        path: P,
        test_name: &str,
    ) -> Result<String> {
        let content = fs::read_to_string(path)?;

        let encoded_content = BASE64_STANDARD.encode(&content);

        let test_name_json = serde_json::to_string(test_name)
            .map_err(|e| Error::Io(std::io::Error::other(format!("bad test_name: {}", e))))?;

        let tracer_code = format!(
            r#"
import sys
import json
import types
import inspect
import functools

# Runtime type tracer with call tracing
class TypeTracer:
    def __init__(self):
        self.traces = {{"variables": {{}}, "functions": {{}}}}
        self.call_stack = []
        self.in_trace = False

    def get_type_name(self, value):
        if value is None:
            return "None"
        elif isinstance(value, bool):
            return "bool"
        elif isinstance(value, int):
            return "int"
        elif isinstance(value, float):
            return "float"
        elif isinstance(value, str):
            return "str"
        elif isinstance(value, bytes):
            return "bytes"
        elif isinstance(value, list):
            if value:
                inner_type = self.get_type_name(value[0])
                return f"List[{{inner_type}}]"
            return "List[Any]"
        elif isinstance(value, dict):
            if value:
                key_type = self.get_type_name(next(iter(value.keys())))
                val_type = self.get_type_name(next(iter(value.values())))
                return f"Dict[{{key_type}}, {{val_type}}]"
            return "Dict[Any, Any]"
        elif isinstance(value, tuple):
            if value:
                types_list = [self.get_type_name(item) for item in value]
                return f"Tuple[{{', '.join(types_list)}}]"
            return "Tuple[()]"
        elif isinstance(value, set):
            if value:
                inner_type = self.get_type_name(next(iter(value)))
                return f"Set[{{inner_type}}]"
            return "Set[Any]"
        else:
            return type(value).__name__

    def trace_function_call(self, func_name, args, result):
        arg_types = [self.get_type_name(arg) for arg in args]
        result_type = self.get_type_name(result)

        if func_name not in self.traces["functions"]:
            self.traces["functions"][func_name] = {{"args": [], "returns": []}}

        self.traces["functions"][func_name]["args"].append(arg_types)
        self.traces["functions"][func_name]["returns"].append(result_type)

    def trace_calls(self, frame, event, arg):
        if self.in_trace:
            return

        self.in_trace = True
        try:
            if event == 'call':
                func_name = frame.f_code.co_name
                if not func_name.startswith('_') and func_name not in ['<module>', 'trace_calls']:
                    # Get function arguments
                    args = []
                    arg_names = frame.f_code.co_varnames[:frame.f_code.co_argcount]
                    for name in arg_names:
                        if name in frame.f_locals and name != 'self':
                            args.append(frame.f_locals[name])

                    self.call_stack.append((func_name, args))

            elif event == 'return':
                if self.call_stack:
                    func_name, args = self.call_stack.pop()
                    if not func_name.startswith('_'):
                        self.trace_function_call(func_name, args, arg)
        finally:
            self.in_trace = False

        return self.trace_calls

    def print_traces(self):
        print("TRACE_OUTPUT_START")
        print(json.dumps(self.traces, indent=2))
        print("TRACE_OUTPUT_END")

_tracer = TypeTracer()

# Execute the original code (safely using base64 encoding)
import base64
exec(base64.b64decode('{encoded_content}').decode('utf-8'))

# Run the specific test function with tracing enabled
current_module = sys.modules[__name__]
TEST_NAME = {test_name}
if hasattr(current_module, TEST_NAME):
    test_func = getattr(current_module, TEST_NAME)
    sys.settrace(_tracer.trace_calls)
    try:
        print('Tracing specific test: {{}}'.format(TEST_NAME))
        test_func()
    except Exception as e:
        print('Error calling {{}}: {{}}'.format(TEST_NAME, str(e)))
    finally:
        sys.settrace(None)

_tracer.print_traces()
"#,
            encoded_content = encoded_content,
            test_name = test_name_json
        );

        Ok(tracer_code)
    }

    /// Parse the trace output from the executed Python code

    fn parse_trace_output(&mut self, output: &str) -> Result<()> {
        // Look for trace output between markers
        if let Some(start) = output.find("TRACE_OUTPUT_START") {
            if let Some(end) = output[start..].find("TRACE_OUTPUT_END") {
                let end = start + end;

                let trace_json = &output[start + "TRACE_OUTPUT_START".len()..end].trim();

                match serde_json::from_str::<serde_json::Value>(trace_json) {
                    Ok(trace_data) => {
                        self.process_trace_data(&trace_data)?;
                    },
                    Err(e) => {
                        log::error!("Failed to parse trace JSON from Python script: {}", e);

                        if self.verbose {
                            eprintln!("-- Problematic JSON --\n{}\n-- End JSON --", trace_json);
                        }
                    },
                }
            } else {
                log::warn!(
                    "Found TRACE_OUTPUT_START but missing TRACE_OUTPUT_END marker in Python output"
                );

                if self.verbose {
                    eprintln!("-- Python Output --\n{}\n-- End Output --", output);
                }
            }
        } else {
            log::warn!(
                "No trace output markers found in Python script output - script may have failed \
                 to execute properly"
            );

            if self.verbose {
                eprintln!("-- Python Output --\n{}\n-- End Output --", output);
            }
        }

        Ok(())
    }

    /// Process the parsed trace data and convert to our Type system

    fn process_trace_data(&mut self, data: &serde_json::Value) -> Result<()> {
        // Process variable traces
        if let Some(variables) = data.get("variables").and_then(|v| v.as_object()) {
            for (var_name, type_list) in variables {
                if let Some(types) = type_list.as_array() {
                    for type_str in types {
                        if let Some(type_name) = type_str.as_str() {
                            let our_type = Self::convert_python_type_to_our_type(type_name);

                            self.traces.add_variable(var_name.clone(), our_type);
                        }
                    }
                }
            }
        }

        // Process function traces
        if let Some(functions) = data.get("functions").and_then(|v| v.as_object()) {
            for (func_name, func_data) in functions {
                if let Some(func_obj) = func_data.as_object() {
                    let args: &[serde_json::Value] = func_obj
                        .get("args")
                        .and_then(|a| a.as_array())
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);

                    let returns: &[serde_json::Value] = func_obj
                        .get("returns")
                        .and_then(|r| r.as_array())
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);

                    for (arg_call, return_call) in args.iter().zip(returns.iter()) {
                        let arg_types: Vec<Type> = arg_call
                            .as_array()
                            .map(Vec::as_slice)
                            .unwrap_or(&[])
                            .iter()
                            .filter_map(|t| t.as_str())
                            .map(Self::convert_python_type_to_our_type)
                            .collect();

                        let return_type = return_call
                            .as_str()
                            .map(Self::convert_python_type_to_our_type)
                            .unwrap_or(Type::Unknown);

                        self.traces
                            .add_function_call(func_name.clone(), arg_types, return_type);
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert Python type string to our Type enum

    fn convert_python_type_to_our_type(type_str: &str) -> Type {
        match type_str {
            "None" => Type::None,
            "bool" => Type::Bool,
            "int" => Type::Int,
            "float" => Type::Float,
            "str" => Type::Str,
            "bytes" => Type::Bytes,
            s if s.starts_with("List[") => {
                let inner = &s[5..s.len() - 1];

                Type::List(Box::new(Self::convert_python_type_to_our_type(inner)))
            },
            s if s.starts_with("Dict[") => {
                let inner = &s[5..s.len() - 1];

                let parts: Vec<&str> = inner.split(", ").collect();

                if parts.len() == 2 {
                    Type::Dict(
                        Box::new(Self::convert_python_type_to_our_type(parts[0])),
                        Box::new(Self::convert_python_type_to_our_type(parts[1])),
                    )
                } else {
                    Type::Dict(Box::new(Type::Any), Box::new(Type::Any))
                }
            },
            s if s.starts_with("Tuple[") => {
                let inner = &s[6..s.len() - 1];

                if inner == "()" {
                    Type::Tuple(vec![])
                } else {
                    let parts: Vec<&str> = inner.split(", ").collect();

                    let types = parts
                        .iter()
                        .map(|&p| Self::convert_python_type_to_our_type(p))
                        .collect();

                    Type::Tuple(types)
                }
            },
            s if s.starts_with("Set[") => {
                let inner = &s[4..s.len() - 1];

                Type::Set(Box::new(Self::convert_python_type_to_our_type(inner)))
            },
            "Any" => Type::Any,
            other => Type::Named(other.to_string()),
        }
    }

    /// Print a summary of collected traces

    fn print_trace_summary(&self) {
        println!("\n=== Runtime Type Trace Summary ===");

        if !self.traces.variables.is_empty() {
            println!("\nVariable Types:");

            for (name, types) in &self.traces.variables {
                let mut unique_types: Vec<String> = types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                unique_types.sort();

                println!("  {}: {}", name, unique_types.join(" | "));
            }
        }

        if !self.traces.functions.is_empty() {
            println!("\nFunction Signatures:");

            for (name, (arg_calls, return_calls)) in &self.traces.functions {
                println!("  {}:", name);

                for (args, ret) in arg_calls.iter().zip(return_calls.iter()) {
                    let arg_strs: Vec<String> = args.iter().map(|t| t.to_string()).collect();

                    println!("    ({}) -> {}", arg_strs.join(", "), ret);
                }
            }
        }

        println!("=== End Trace Summary ===\n");
    }

    /// Returns the collected type traces.

    pub fn into_traces(self) -> TypeTrace {
        self.traces
    }

    /// Get a reference to the collected traces

    pub fn traces(&self) -> &TypeTrace {
        &self.traces
    }
}

#[cfg(test)]

mod tests {

    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]

    fn test_tracer_initialization() {
        let tracer = RuntimeTracer::new(false);

        assert!(!tracer.verbose);
    }

    #[test]

    fn test_type_trace_operations() {
        let mut trace = TypeTrace::default();

        // Test variable tracing
        trace.add_variable("x".to_string(), Type::Int);

        trace.add_variable("x".to_string(), Type::Str);

        let x_types = trace.get_variable_types("x");

        assert_eq!(x_types.len(), 2);

        assert!(x_types.contains(&&Type::Int));

        assert!(x_types.contains(&&Type::Str));

        // Test function tracing
        trace.add_function_call("test_func".to_string(), vec![Type::Int, Type::Str], Type::Bool);

        assert!(trace.functions.contains_key("test_func"));

        let (args, returns) = &trace.functions["test_func"];

        assert_eq!(args.len(), 1);

        assert_eq!(returns.len(), 1);

        assert_eq!(args[0], vec![Type::Int, Type::Str]);

        assert_eq!(returns[0], Type::Bool);
    }

    #[test]

    fn test_get_variable_types_deduplication() {
        let mut trace = TypeTrace::default();

        // Add duplicate types for the same variable
        trace.add_variable("y".to_string(), Type::Int);

        trace.add_variable("y".to_string(), Type::Int); // duplicate
        trace.add_variable("y".to_string(), Type::Str);

        trace.add_variable("y".to_string(), Type::Int); // another duplicate
        trace.add_variable("y".to_string(), Type::Str); // another duplicate

        let y_types = trace.get_variable_types("y");

        // Should only have 2 unique types despite 5 additions
        assert_eq!(y_types.len(), 2);

        assert!(y_types.contains(&&Type::Int));

        assert!(y_types.contains(&&Type::Str));

        // Test with complex types
        trace.add_variable("z".to_string(), Type::List(Box::new(Type::Int)));

        trace.add_variable("z".to_string(), Type::List(Box::new(Type::Int))); // duplicate
        trace.add_variable("z".to_string(), Type::List(Box::new(Type::Str)));

        let z_types = trace.get_variable_types("z");

        assert_eq!(z_types.len(), 2);

        assert!(z_types.contains(&&Type::List(Box::new(Type::Int))));

        assert!(z_types.contains(&&Type::List(Box::new(Type::Str))));
    }

    #[test]

    fn test_python_type_conversion() {
        assert_eq!(RuntimeTracer::convert_python_type_to_our_type("int"), Type::Int);

        assert_eq!(RuntimeTracer::convert_python_type_to_our_type("str"), Type::Str);

        assert_eq!(RuntimeTracer::convert_python_type_to_our_type("None"), Type::None);

        // Test complex types
        let list_type = RuntimeTracer::convert_python_type_to_our_type("List[int]");

        assert_eq!(list_type, Type::List(Box::new(Type::Int)));

        let dict_type = RuntimeTracer::convert_python_type_to_our_type("Dict[str, int]");

        assert_eq!(dict_type, Type::Dict(Box::new(Type::Str), Box::new(Type::Int)));
    }

    #[test]

    fn test_instrumentation_creation() {
        let tracer = RuntimeTracer::new(false);

        // Create a simple test Python file
        let test_content = r#"
def simple_function(x):
    return x + 1

def test_simple():
    assert simple_function(5) == 6
"#;

        let temp_file = NamedTempFile::with_suffix(".py").unwrap();

        fs::write(temp_file.path(), test_content).unwrap();

        let instrumented = tracer.instrument_python_file(temp_file.path());

        assert!(instrumented.is_ok());

        let content = instrumented.unwrap();

        assert!(content.contains("TypeTracer"));

        assert!(content.contains("TRACE_OUTPUT_START"));

        assert!(content.contains("sys.settrace"));

        // temp_file is automatically cleaned up when it goes out of scope
    }
}
