//! Type system definitions for omnitype.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    fmt,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

/// A type variable used during type inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]

pub struct TypeVar(pub u32);

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T{}", self.0)
    }
}

/// Represents a type in the omnitype system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]

pub enum Type {
    /// The unknown type (used during inference)
    #[default]
    Unknown,

    /// The `None` type
    None,

    /// The `Any` type (top type)
    Any,

    /// Boolean type (true/false)
    Bool,

    /// Integer number type
    Int,

    /// Floating-point number type
    Float,

    /// Unicode string type
    Str,

    /// Binary data type
    Bytes,

    /// Homogeneous list/array type
    List(Box<Type>),

    /// Dictionary/map type with key and value types
    Dict(Box<Type>, Box<Type>), // key type, value type

    /// Fixed-size heterogeneous sequence type
    Tuple(Vec<Type>),

    /// Unordered collection of unique elements
    Set(Box<Type>),

    /// Function type with parameter and return types
    Function {
        /// List of parameter types
        params: Vec<Type>,
        /// Return type
        returns: Box<Type>,
    },

    /// Union type representing one of several possible types (T1 | T2 | ...)
    Union(Vec<Type>),

    /// Type variable used during type inference
    Var(TypeVar),

    /// Named type (e.g., user-defined class or type alias)
    Named(String),

    /// Generic type with type parameters
    Generic {
        /// Name of the generic type
        name: String,
        /// Type parameters
        params: Vec<Type>,
    },
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);

        match self {
            Type::List(inner) => inner.hash(state),
            Type::Dict(k, v) => {
                k.hash(state);

                v.hash(state);
            },
            Type::Tuple(types) => types.hash(state),
            Type::Set(inner) => inner.hash(state),
            Type::Function { params, returns } => {
                params.hash(state);

                returns.hash(state);
            },
            Type::Union(types) => types.hash(state),
            Type::Var(var) => var.hash(state),
            Type::Named(name) => name.hash(state),
            Type::Generic { name, params } => {
                name.hash(state);

                params.hash(state);
            },
            _ => (),
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::List(a), Type::List(b)) => a == b,
            (Type::Dict(ak, av), Type::Dict(bk, bv)) => ak == bk && av == bv,
            (Type::Tuple(a), Type::Tuple(b)) => a == b,
            (Type::Set(a), Type::Set(b)) => a == b,
            (
                Type::Function { params: a_params, returns: a_ret },
                Type::Function { params: b_params, returns: b_ret },
            ) => a_params == b_params && a_ret == b_ret,
            (Type::Union(a), Type::Union(b)) => a == b,
            (Type::Var(a), Type::Var(b)) => a == b,
            (Type::Named(a), Type::Named(b)) => a == b,
            (
                Type::Generic { name: a_name, params: a_params },
                Type::Generic { name: b_name, params: b_params },
            ) => a_name == b_name && a_params == b_params,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
}

impl Eq for Type {}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Type::Unknown, Type::Unknown) => Ordering::Equal,
            (Type::None, Type::None) => Ordering::Equal,
            (Type::Any, Type::Any) => Ordering::Equal,
            (Type::Bool, Type::Bool) => Ordering::Equal,
            (Type::Int, Type::Int) => Ordering::Equal,
            (Type::Float, Type::Float) => Ordering::Equal,
            (Type::Str, Type::Str) => Ordering::Equal,
            (Type::Bytes, Type::Bytes) => Ordering::Equal,
            (Type::List(a), Type::List(b)) => a.cmp(b),
            (Type::Dict(ak, av), Type::Dict(bk, bv)) => match ak.cmp(bk) {
                Ordering::Equal => av.cmp(bv),
                ord => ord,
            },
            (Type::Tuple(a), Type::Tuple(b)) => a.cmp(b),
            (Type::Set(a), Type::Set(b)) => a.cmp(b),
            (
                Type::Function { params: a_params, returns: a_ret },
                Type::Function { params: b_params, returns: b_ret },
            ) => match a_params.cmp(b_params) {
                Ordering::Equal => a_ret.cmp(b_ret),
                ord => ord,
            },
            (Type::Union(a), Type::Union(b)) => a.cmp(b),
            (Type::Var(a), Type::Var(b)) => a.0.cmp(&b.0),
            (Type::Named(a), Type::Named(b)) => a.cmp(b),
            (
                Type::Generic { name: a_name, params: a_params },
                Type::Generic { name: b_name, params: b_params },
            ) => match a_name.cmp(b_name) {
                Ordering::Equal => a_params.cmp(b_params),
                ord => ord,
            },
            (a, b) => {
                // Compare discriminants by matching all possible variants
                match (a, b) {
                    (Type::Unknown, _) => Ordering::Less,
                    (_, Type::Unknown) => Ordering::Greater,
                    (Type::None, _) => Ordering::Less,
                    (_, Type::None) => Ordering::Greater,
                    (Type::Any, _) => Ordering::Less,
                    (_, Type::Any) => Ordering::Greater,
                    (Type::Bool, _) => Ordering::Less,
                    (_, Type::Bool) => Ordering::Greater,
                    (Type::Int, _) => Ordering::Less,
                    (_, Type::Int) => Ordering::Greater,
                    (Type::Float, _) => Ordering::Less,
                    (_, Type::Float) => Ordering::Greater,
                    (Type::Str, _) => Ordering::Less,
                    (_, Type::Str) => Ordering::Greater,
                    (Type::Bytes, _) => Ordering::Less,
                    (_, Type::Bytes) => Ordering::Greater,
                    (Type::List(_), _) => Ordering::Less,
                    (_, Type::List(_)) => Ordering::Greater,
                    (Type::Dict(_, _), _) => Ordering::Less,
                    (_, Type::Dict(_, _)) => Ordering::Greater,
                    (Type::Tuple(_), _) => Ordering::Less,
                    (_, Type::Tuple(_)) => Ordering::Greater,
                    (Type::Set(_), _) => Ordering::Less,
                    (_, Type::Set(_)) => Ordering::Greater,
                    (Type::Function { .. }, _) => Ordering::Less,
                    (_, Type::Function { .. }) => Ordering::Greater,
                    (Type::Union(_), _) => Ordering::Less,
                    (_, Type::Union(_)) => Ordering::Greater,
                    (Type::Var(_), _) => Ordering::Less,
                    (_, Type::Var(_)) => Ordering::Greater,
                    (Type::Named(_), _) => Ordering::Less,
                    (_, Type::Named(_)) => Ordering::Greater,
                    (Type::Generic { .. }, _) => Ordering::Equal,
                }
            },
        }
    }
}

impl Type {
    /// Creates a normalized union type by sorting and deduplicating the input
    /// types. This ensures that the same set of types always produces the
    /// same union, regardless of the order of input types.

    pub fn union_of(types: Vec<Type>) -> crate::error::Result<Type> {
        if types.is_empty() {
            return Ok(Type::Unknown);
        }

        // Flatten nested unions and collect unique types
        let mut unique_types = BTreeSet::new();

        for ty in types {
            match ty {
                Type::Union(nested_types) => {
                    for nested_ty in nested_types {
                        unique_types.insert(nested_ty);
                    }
                },
                _ => {
                    unique_types.insert(ty);
                },
            }
        }

        // If there's only one unique type, return it directly
        if unique_types.len() == 1 {
            return unique_types.into_iter().next().ok_or_else(|| {
                crate::error::Error::type_error("No unique type found".to_string())
            });
        }

        // Convert back to a sorted vector
        Ok(Type::Union(unique_types.into_iter().collect()))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unknown => write!(f, "Unknown"),
            Type::None => write!(f, "None"),
            Type::Any => write!(f, "Any"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Str => write!(f, "str"),
            Type::Bytes => write!(f, "bytes"),
            Type::List(inner) => write!(f, "List[{}]", inner),
            Type::Dict(k, v) => write!(f, "Dict[{}, {}]", k, v),
            Type::Tuple(items) => {
                let items_str = items
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "Tuple[{}]", items_str)
            },
            Type::Set(inner) => write!(f, "Set[{}]", inner),
            Type::Function { params, returns } => {
                let params_str = params
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "Callable[[{}], {}]", params_str, returns)
            },
            Type::Union(types) => {
                let types_str = types
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ");

                write!(f, "{}", types_str)
            },
            Type::Var(var) => write!(f, "{}", var),
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic { name, params } => {
                let params_str = params
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{}[{}]", name, params_str)
            },
        }
    }
}

/// Type environment that maps variable names to their types.
#[derive(Debug, Default, Clone)]
#[allow(clippy::empty_line_after_outer_attr)]

pub struct TypeEnv {
    bindings: HashMap<String, Type>,
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    /// Creates a new empty type environment.

    pub fn new() -> Self {
        Self { bindings: HashMap::new(), parent: None }
    }

    /// Creates a new nested type environment.

    pub fn nested(env: TypeEnv) -> Self {
        Self { bindings: HashMap::new(), parent: Some(Box::new(env)) }
    }

    /// Looks up a variable in the environment.

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }

    /// Binds a variable to a type in the current scope.

    pub fn bind(&mut self, name: String, ty: Type) -> Option<Type> {
        self.bindings.insert(name, ty)
    }

    /// Returns the parent environment, if any.

    pub fn parent(&self) -> Option<&TypeEnv> {
        self.parent.as_deref()
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_type_display() {
        assert_eq!(Type::Int.to_string(), "int");

        assert_eq!(Type::List(Box::new(Type::Int)).to_string(), "List[int]");

        assert_eq!(
            Type::Dict(Box::new(Type::Str), Box::new(Type::Int)).to_string(),
            "Dict[str, int]"
        );

        assert_eq!(
            Type::Function { params: vec![Type::Int, Type::Str], returns: Box::new(Type::Bool) }
                .to_string(),
            "Callable[[int, str], bool]"
        );
    }

    #[test]

    fn test_type_env() {
        let mut env = TypeEnv::new();

        env.bind("x".to_string(), Type::Int);

        assert_eq!(env.lookup("x"), Some(&Type::Int));

        assert_eq!(env.lookup("y"), None);

        let mut inner_env = TypeEnv::nested(env);

        inner_env.bind("y".to_string(), Type::Str);

        assert_eq!(inner_env.lookup("x"), Some(&Type::Int));

        assert_eq!(inner_env.lookup("y"), Some(&Type::Str));
    }
}
