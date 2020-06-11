use crate::object_tree::*;
use crate::parser::{Spanned, SyntaxNode};
use crate::{diagnostics::Diagnostics, typeregister::Type};
use core::cell::RefCell;
use std::rc::Weak;

/// The Expression is hold by properties, so it should not hold any strong references to node from the object_tree
#[derive(Debug, Clone)]
pub enum Expression {
    /// Something went wrong (and an error will be reported)
    Invalid,
    /// We haven't done the lookup yet
    Uncompiled(SyntaxNode),
    /// A string literal. The .0 is the content of the string, without the quotes
    StringLiteral(String),
    /// Number
    NumberLiteral(f64),

    /// Reference to the signal <name> in the <element> within the <Component>
    ///
    /// Note: if we are to separate expression and statement, we probably do not need to have signal reference within expressions
    SignalReference {
        component: Weak<Component>,
        element: Weak<RefCell<Element>>,
        name: String,
    },

    /// Reference to the signal <name> in the <element> within the <Component>
    ///
    /// Note: if we are to separate expression and statement, we probably do not need to have signal reference within expressions
    PropertyReference {
        component: Weak<Component>,
        element: Weak<RefCell<Element>>,
        name: String,
    },

    /// Cast an expression to the given type
    Cast {
        from: Box<Expression>,
        to: Type,
    },

    /// a code block with different expression
    CodeBlock(Vec<Expression>),

    /// A function call
    FunctionCall {
        function: Box<Expression>,
    },

    SelfAssignment {
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        /// '+', '-', '/', or '*'
        op: char,
    },

    ResourceReference {
        absolute_source_path: String,
    },

    Condition {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
    },
}

impl Expression {
    /// Return the type of this property
    pub fn ty(&self) -> Type {
        match self {
            Expression::Invalid => Type::Invalid,
            Expression::Uncompiled(_) => Type::Invalid,
            Expression::StringLiteral(_) => Type::String,
            Expression::NumberLiteral(_) => Type::Float32,
            Expression::SignalReference { .. } => Type::Signal,
            Expression::PropertyReference { element, name, .. } => {
                element.upgrade().unwrap().borrow().lookup_property(name)
            }
            Expression::Cast { to, .. } => to.clone(),
            Expression::CodeBlock(sub) => sub.last().map_or(Type::Invalid, |e| e.ty()),
            Expression::FunctionCall { function } => function.ty(),
            Expression::SelfAssignment { .. } => Type::Invalid,
            Expression::ResourceReference { .. } => Type::Resource,
            Expression::Condition { condition: _, true_expr, false_expr } => {
                let true_type = true_expr.ty();
                let false_type = false_expr.ty();
                if true_type == false_type {
                    true_type
                } else {
                    Type::Invalid
                }
            }
        }
    }

    /// Call the visitor for each sub-expression.  (note: this function does not recurse)
    pub fn visit(&self, mut visitor: impl FnMut(&Self)) {
        match self {
            Expression::Invalid => {}
            Expression::Uncompiled(_) => {}
            Expression::StringLiteral(_) => {}
            Expression::NumberLiteral(_) => {}
            Expression::SignalReference { .. } => {}
            Expression::PropertyReference { .. } => {}
            Expression::Cast { from, .. } => visitor(&**from),
            Expression::CodeBlock(sub) => {
                for e in sub {
                    visitor(e)
                }
            }
            Expression::FunctionCall { function } => visitor(&**function),
            Expression::SelfAssignment { lhs, rhs, .. } => {
                visitor(&**lhs);
                visitor(&**rhs);
            }
            Expression::ResourceReference { .. } => {}
            Expression::Condition { condition, true_expr, false_expr } => {
                visitor(&**condition);
                visitor(&**true_expr);
                visitor(&**false_expr);
            }
        }
    }

    pub fn visit_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
        match self {
            Expression::Invalid => {}
            Expression::Uncompiled(_) => {}
            Expression::StringLiteral(_) => {}
            Expression::NumberLiteral(_) => {}
            Expression::SignalReference { .. } => {}
            Expression::PropertyReference { .. } => {}
            Expression::Cast { from, .. } => visitor(&mut **from),
            Expression::CodeBlock(sub) => {
                for e in sub {
                    visitor(e)
                }
            }
            Expression::FunctionCall { function } => visitor(&mut **function),
            Expression::SelfAssignment { lhs, rhs, .. } => {
                visitor(&mut **lhs);
                visitor(&mut **rhs);
            }
            Expression::ResourceReference { .. } => {}
            Expression::Condition { condition, true_expr, false_expr } => {
                visitor(&mut **condition);
                visitor(&mut **true_expr);
                visitor(&mut **false_expr);
            }
        }
    }

    pub fn is_constant(&self) -> bool {
        match self {
            Expression::Invalid => true,
            Expression::Uncompiled(_) => false,
            Expression::StringLiteral(_) => true,
            Expression::NumberLiteral(_) => true,
            Expression::SignalReference { .. } => false,
            Expression::PropertyReference { .. } => false,
            Expression::Cast { from, .. } => from.is_constant(),
            Expression::CodeBlock(sub) => sub.len() == 1 && sub.first().unwrap().is_constant(),
            Expression::FunctionCall { .. } => false,
            Expression::SelfAssignment { .. } => false,
            Expression::ResourceReference { .. } => true,
            Expression::Condition { .. } => false,
        }
    }

    pub fn cast_boxed(self: Box<Self>, target_type: Type) -> Box<Expression> {
        Box::new(Expression::Cast { from: self, to: target_type })
    }

    /// Create a conversion node if needed, or throw an error if the type is not matching
    pub fn maybe_convert_to(
        self,
        target_type: Type,
        node: &SyntaxNode,
        diag: &mut Diagnostics,
    ) -> Expression {
        match self {
            Expression::Condition { condition, true_expr, false_expr } => {
                let true_expr = if true_expr.ty() != target_type {
                    true_expr.cast_boxed(target_type.clone())
                } else {
                    true_expr
                };
                let false_expr = if false_expr.ty() != target_type {
                    false_expr.cast_boxed(target_type)
                } else {
                    false_expr
                };
                Expression::Condition { condition, true_expr, false_expr }
            }
            _ => {
                let ty = self.ty();
                if ty == target_type {
                    self
                } else if ty.can_convert(&target_type) {
                    Expression::Cast { from: Box::new(self), to: target_type }
                } else if ty == Type::Invalid {
                    self
                } else {
                    diag.push_error(
                        format!("Cannot convert {} to {}", ty, target_type),
                        node.span(),
                    );
                    self
                }
            }
        }
    }
}
