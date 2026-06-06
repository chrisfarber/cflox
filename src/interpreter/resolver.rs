use std::collections::HashMap;

use crate::parser::{
    ast::{
        Declaration, DeclarationKind, Expression, ExpressionKind, Function, Statement,
        StatementKind, Unary,
    },
    diagnostic::Diagnostic,
    node::{Node, NodeId, Span},
};

#[derive(Debug)]
pub struct Resolution {
    depth: Option<u32>,
}

#[derive(Debug)]
pub struct Resolutions {
    map: HashMap<NodeId, Resolution>,
}

impl Resolutions {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn resolve<T>(&self, node: &Node<T>) -> Option<u32> {
        self.map.get(&node.id()).and_then(|r| r.depth)
    }

    fn set<T>(&mut self, node: &Node<T>, distance: u32) {
        self.map.insert(
            node.id(),
            Resolution {
                depth: Some(distance),
            },
        );
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum FunctionType {
    None,
    Function,
}

#[derive(Debug)]
struct Resolver<'a> {
    mapping: &'a mut Resolutions,
    scopes: Vec<HashMap<String, bool>>,
    current_function: FunctionType,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
    pub fn new(resolutions: &'a mut Resolutions) -> Self {
        Self {
            mapping: resolutions,
            scopes: vec![],
            current_function: FunctionType::None,
            diagnostics: vec![],
        }
    }

    pub fn resolve(&mut self, program: &[Declaration]) {
        self.resolve_declarations(program);
    }

    fn resolve_declarations(&mut self, decls: &[Declaration]) {
        for decl in decls {
            self.resolve_declaration(decl);
        }
    }

    fn resolve_declaration(&mut self, decl: &Declaration) {
        match &decl.node {
            DeclarationKind::Statement(stmt) => self.resolve_statement(stmt),
            DeclarationKind::Function(fdecl) => self.resolve_function_declaration(decl.span, fdecl),
            DeclarationKind::Var {
                identifier,
                initial,
            } => self.resolve_var_declaration(decl.span, identifier, initial),
        }
    }

    fn resolve_statement(&mut self, stmt: &Statement) {
        match &stmt.node {
            StatementKind::Block(decls) => self.resolve_block(decls),
            StatementKind::Expression(expr) => self.resolve_expression(expr),
            StatementKind::Print(expr) => self.resolve_expression(expr),
            StatementKind::Return(expr) => {
                if self.current_function == FunctionType::None {
                    self.diagnostics
                        .push(Diagnostic::error(stmt, "Can't return from top-level code."))
                }
                if let Some(expr) = expr {
                    self.resolve_expression(expr);
                }
            }
            StatementKind::While { condition, body } => {
                self.resolve_expression(condition);
                self.resolve_statement(body);
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression(condition);
                self.resolve_statement(then_branch);
                if let Some(else_stm) = else_branch {
                    self.resolve_statement(else_stm);
                }
            }
        }
    }

    fn resolve_block(&mut self, decls: &[Declaration]) {
        self.begin_scope();
        self.resolve_declarations(decls);
        self.end_scope();
    }

    fn resolve_function_declaration(&mut self, span: Span, fdecl: &Function) {
        self.declare(span, &fdecl.name);
        self.define(&fdecl.name);

        let enclosing_function = self.current_function;
        self.current_function = FunctionType::Function;
        self.begin_scope();
        for (param_span, param) in &fdecl.parameter_names {
            self.declare(*param_span, param);
            self.define(param);
        }
        let StatementKind::Block(decls) = &fdecl.body.node else {
            unreachable!("function bodies are always blocks");
        };
        self.resolve_declarations(decls);
        self.end_scope();
        self.current_function = enclosing_function;
    }

    fn resolve_var_declaration(
        &mut self,
        span: Span,
        identifier: &str,
        initial: &Option<Expression>,
    ) {
        self.declare(span, identifier);
        if let Some(initializer) = initial {
            self.resolve_expression(initializer);
        }
        self.define(identifier);
    }

    fn resolve_expression(&mut self, expr: &Expression) {
        match &expr.node {
            ExpressionKind::Variable(ident) => {
                if let Some(scope) = self.scopes.last()
                    && scope.get(ident) == Some(&false)
                {
                    self.diagnostics.push(Diagnostic::error(
                        expr.span,
                        "Can't read local variable in its own initializer.",
                    ));
                }
                self.resolve_local(expr, ident);
            }
            ExpressionKind::Assign(name, value_expr) => {
                self.resolve_expression(value_expr);
                self.resolve_local(expr, name);
            }
            ExpressionKind::Call(callee, args) => {
                self.resolve_expression(callee);
                self.resolve_expressions(args);
            }
            ExpressionKind::Logical(logical) => {
                self.resolve_expression(&logical.left);
                self.resolve_expression(&logical.right);
            }
            ExpressionKind::Binary(binary) => {
                self.resolve_expression(&binary.left);
                self.resolve_expression(&binary.right);
            }
            ExpressionKind::Unary(unary) => match unary {
                Unary::Negate(expr) | Unary::Not(expr) => self.resolve_expression(expr),
            },
            ExpressionKind::Literal(_) => {}
        }
    }

    fn resolve_expressions(&mut self, exprs: &[Expression]) {
        for expr in exprs {
            self.resolve_expression(expr);
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    /// Note that we are about to define a var
    fn declare(&mut self, span: Span, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                self.diagnostics.push(Diagnostic::error(
                    span,
                    "Already a variable with this name in this scope.",
                ))
            }
            scope.insert(name.to_owned(), false);
        }
    }

    /// Mark a var as defined
    fn define(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), true);
        }
    }

    /// set the distance for a var expression to its scope
    fn resolve_local(&mut self, expr: &Expression, name: &str) {
        for (distance, scope) in self.scopes.iter().rev().enumerate() {
            if scope.contains_key(name) {
                self.mapping.set(expr, distance as u32);
                return;
            }
        }
    }
}

pub fn resolve(resolutions: &mut Resolutions, program: &[Declaration]) -> Vec<Diagnostic> {
    let mut resolver = Resolver::new(resolutions);
    resolver.resolve(program);
    resolver.diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{diagnostic::has_error, parse_str};

    /// Walk into a single-statement block and return the declarations it holds.
    fn block_decls(decl: &Declaration) -> &[Declaration] {
        let DeclarationKind::Statement(stmt) = &decl.node else {
            panic!("expected a statement declaration");
        };
        let StatementKind::Block(decls) = &stmt.node else {
            panic!("expected a block statement");
        };
        decls
    }

    /// Pull the variable expression out of a `print <var>;` declaration.
    fn print_variable(decl: &Declaration) -> &Expression {
        let DeclarationKind::Statement(stmt) = &decl.node else {
            panic!("expected a statement declaration");
        };
        let StatementKind::Print(expr) = &stmt.node else {
            panic!("expected a print statement");
        };
        expr
    }

    #[test]
    fn shadowed_variable_resolves_to_nearest_scope() {
        // The inner `print a` must resolve to the inner `a` (distance 0),
        // not the outer one.
        let source = "{ var a = \"outer\"; { var a = \"inner\"; print a; } }";
        let (decls, diags) = parse_str(source);
        assert!(!has_error(&diags), "{diags:?}");

        let mut resolutions = Resolutions::new();
        let res_diags = resolve(&mut resolutions, &decls);
        assert!(res_diags.is_empty(), "{res_diags:?}");

        // decls[0] is the outer block; its second decl is the inner block;
        // the inner block's second decl is `print a`.
        let outer = block_decls(&decls[0]);
        let inner = block_decls(&outer[1]);
        let var_expr = print_variable(&inner[1]);

        assert_eq!(resolutions.resolve(var_expr), Some(0));
    }
}
