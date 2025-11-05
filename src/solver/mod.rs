//! Constraint-based type inference engine.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::collections::HashMap;

use crate::{
    error::Result,
    types::{Type, TypeVar},
};

/// Represents a constraint between two types.
#[derive(Debug, Clone, PartialEq, Eq)]

pub enum Constraint {
    /// Type equality constraint: T1 = T2
    Equal(Type, Type),

    /// Subtype constraint: T1 <: T2
    Subtype(Type, Type),

    /// Occurs check constraint (prevents infinite types)
    Occurs(TypeVar, Type),
}

/// A constraint solver for type inference.
#[allow(clippy::empty_line_after_outer_attr)]

pub struct ConstraintSolver {
    /// Type variable counter for generating fresh type variables
    next_var: u32,

    /// Set of constraints to solve
    constraints: Vec<Constraint>,

    /// Substitution mapping from type variables to types
    substitution: HashMap<TypeVar, Type>,
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintSolver {
    /// Creates a new constraint solver.

    pub fn new() -> Self {
        Self { next_var: 0, constraints: Vec::new(), substitution: HashMap::new() }
    }

    /// Generates a fresh type variable.

    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);

        self.next_var += 1;

        var
    }

    /// Adds a new constraint to the solver.

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Solves the collected constraints and returns the substitution.

    pub fn solve(mut self) -> Result<HashMap<TypeVar, Type>> {
        while let Some(constraint) = self.constraints.pop() {
            self.solve_constraint(constraint)?;
        }

        Ok(self.substitution)
    }

    /// Solves a single constraint.

    fn solve_constraint(&mut self, constraint: Constraint) -> Result<()> {
        match constraint {
            Constraint::Equal(t1, t2) => self.unify(t1, t2),
            Constraint::Subtype(t1, t2) => self.subtype(t1, t2),
            Constraint::Occurs(var, ty) => {
                if self.occurs_check(var, &ty)? {
                    Err(crate::error::Error::type_error("Occurs check failed".to_string()))
                } else {
                    Ok(())
                }
            },
        }
    }

    /// Unifies two types.

    fn unify(&mut self, t1: Type, t2: Type) -> Result<()> {
        match (t1, t2) {
            (Type::Var(v1), Type::Var(v2)) if v1 == v2 => Ok(()),
            (Type::Var(v), t) | (t, Type::Var(v)) => {
                if self.occurs_check(v, &t)? {
                    return Err(crate::error::Error::type_error("Occurs check failed".to_string()));
                }

                self.substitution.insert(v, t);

                Ok(())
            },
            (Type::Int, Type::Int) | (Type::Str, Type::Str) | (Type::Bool, Type::Bool) => Ok(()),
            (Type::List(a), Type::List(b)) => self.unify(*a, *b),
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                self.unify(*k1, *k2)?;

                self.unify(*v1, *v2)
            },
            (Type::Union(types1), Type::Union(types2)) => {
                // Simple union unification: check if one is subset of other
                if types1.iter().all(|t| types2.contains(t))
                    || types2.iter().all(|t| types1.contains(t))
                {
                    Ok(())
                } else {
                    Err(crate::error::Error::type_error("Union types do not unify".to_string()))
                }
            },
            _ => Err(crate::error::Error::type_error("Types do not unify".to_string())),
        }
    }

    /// Handles subtyping relationships.

    #[allow(clippy::only_used_in_recursion)]
    fn subtype(&mut self, t1: Type, t2: Type) -> Result<()> {
        match (t1, t2) {
            (Type::Int, Type::Int) | (Type::Str, Type::Str) | (Type::Bool, Type::Bool) => Ok(()),
            (Type::List(a), Type::List(b)) => self.subtype(*a, *b),
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                self.subtype(*k1, *k2)?;

                self.subtype(*v1, *v2)
            },
            _ => Err(crate::error::Error::type_error("Subtyping not supported".to_string())),
        }
    }

    /// Performs the occurs check to prevent infinite types.

    #[allow(clippy::only_used_in_recursion)]
    fn occurs_check(&self, var: TypeVar, ty: &Type) -> Result<bool> {
        match ty {
            Type::Var(v) if *v == var => Ok(true),
            Type::List(inner) => self.occurs_check(var, inner),
            Type::Dict(key, val) => {
                if self.occurs_check(var, key)? || self.occurs_check(var, val)? {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            Type::Union(types) => {
                for t in types {
                    if self.occurs_check(var, t)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            },
            _ => Ok(false),
        }
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_fresh_var() {
        let mut solver = ConstraintSolver::new();

        let var1 = solver.fresh_var();

        let var2 = solver.fresh_var();

        assert_ne!(var1, var2);
    }
}
