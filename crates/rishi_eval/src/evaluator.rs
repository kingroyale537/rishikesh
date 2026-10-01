use crate::builtins::register_builtins;
use crate::env::Environment;
use crate::value::{EvalError, FnValue, StructDef, StructInstance, Value};
use rishi_ast::*;
use rishi_lexer::Lexer;
use rishi_parser::Parser;
use rishi_span::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum ControlFlow {
    Normal(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Evaluator {
    pub env: Rc<RefCell<Environment>>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    pub fn new() -> Self {
        let mut global_env = Environment::new();
        register_builtins(&mut global_env);
        Self {
            env: Rc::new(RefCell::new(global_env)),
        }
    }

    pub fn eval_program(&mut self, program: &Program) -> Result<Value, EvalError> {
        let mut last_val = Value::None;
        for stmt in &program.statements {
            match self.eval_stmt(stmt)? {
                ControlFlow::Normal(v) => last_val = v,
                ControlFlow::Return(v) => return Ok(v),
                ControlFlow::Break => {
                    return Err(EvalError::Generic {
                        message: "Break outside of loop".to_string(),
                        span: stmt.span,
                    });
                }
                ControlFlow::Continue => {
                    return Err(EvalError::Generic {
                        message: "Continue outside of loop".to_string(),
                        span: stmt.span,
                    });
                }
            }
        }
        Ok(last_val)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, EvalError> {
        match &stmt.kind {
            StmtKind::Let { name, init, .. } => {
                let val = self.eval_expr(init)?;
                self.env.borrow_mut().define_let(name.clone(), val);
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::Mut { name, init, .. } => {
                let val = self.eval_expr(init)?;
                self.env.borrow_mut().define_mut(name.clone(), val);
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::Assign { target, op, value } => {
                let rhs = self.eval_expr(value)?;
                self.handle_assign(target, *op, rhs, stmt.span)?;
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::FnDecl {
                name,
                params,
                body,
                ..
            } => {
                let fn_val = FnValue {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    env: Rc::clone(&self.env),
                };
                self.env
                    .borrow_mut()
                    .define_let(name.clone(), Value::Function(fn_val));
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::StructDecl {
                name,
                fields,
                methods,
            } => {
                let mut method_map = HashMap::new();
                for m in methods {
                    if let StmtKind::FnDecl {
                        name: m_name,
                        params,
                        body,
                        ..
                    } = &m.kind
                    {
                        method_map.insert(
                            m_name.clone(),
                            FnValue {
                                name: m_name.clone(),
                                params: params.clone(),
                                body: body.clone(),
                                env: Rc::clone(&self.env),
                            },
                        );
                    }
                }

                let struct_def = StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                    methods: method_map,
                };
                self.env
                    .borrow_mut()
                    .define_let(name.clone(), Value::Struct(struct_def));
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::Return(expr_opt) => {
                let val = if let Some(expr) = expr_opt {
                    self.eval_expr(expr)?
                } else {
                    Value::None
                };
                Ok(ControlFlow::Return(val))
            }
            StmtKind::If {
                condition,
                then_body,
                elif_branches,
                else_body,
            } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    return self.eval_block(then_body);
                }

                for (elif_cond, elif_body) in elif_branches {
                    let elif_val = self.eval_expr(elif_cond)?;
                    if elif_val.is_truthy() {
                        return self.eval_block(elif_body);
                    }
                }

                if let Some(else_b) = else_body {
                    return self.eval_block(else_b);
                }

                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::While { condition, body } => {
                while self.eval_expr(condition)?.is_truthy() {
                    match self.eval_block(body)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                    }
                }
                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::ForIn { var, iter, body } => {
                let iter_val = self.eval_expr(iter)?;
                let items: Vec<Value> = match iter_val {
                    Value::List(l) => l.borrow().clone(),
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    other => {
                        return Err(EvalError::TypeError {
                            message: format!("`{}` is not iterable", other.type_name()),
                            span: iter.span,
                        })
                    }
                };

                for item in items {
                    let prev_env = Rc::clone(&self.env);
                    let mut loop_env = Environment::with_parent(Rc::clone(&self.env));
                    loop_env.define_mut(var.clone(), item);
                    self.env = Rc::new(RefCell::new(loop_env));

                    let res = self.eval_block(body);
                    self.env = prev_env;

                    match res? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                    }
                }

                Ok(ControlFlow::Normal(Value::None))
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Continue => Ok(ControlFlow::Continue),
            StmtKind::Expr(expr) => {
                let v = self.eval_expr(expr)?;
                Ok(ControlFlow::Normal(v))
            }
            StmtKind::Import { .. } => {
                // Modules handled as namespace stubs in v0.1
                Ok(ControlFlow::Normal(Value::None))
            }
        }
    }

    pub fn eval_block(&mut self, stmts: &[Stmt]) -> Result<ControlFlow, EvalError> {
        let prev_env = Rc::clone(&self.env);
        self.env = Rc::new(RefCell::new(Environment::with_parent(Rc::clone(&prev_env))));

        let mut res = ControlFlow::Normal(Value::None);
        for stmt in stmts {
            res = self.eval_stmt(stmt)?;
            if !matches!(res, ControlFlow::Normal(_)) {
                break;
            }
        }

        self.env = prev_env;
        Ok(res)
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(i) => Ok(Value::Int(*i)),
                Literal::Float(f) => Ok(Value::Float(*f)),
                Literal::String(s) => self.interpolate_string(s, expr.span),
                Literal::Bool(b) => Ok(Value::Bool(*b)),
                Literal::None => Ok(Value::None),
            },
            ExprKind::Identifier(name) => {
                self.env
                    .borrow()
                    .get(name)
                    .ok_or_else(|| EvalError::UndefinedVariable {
                        name: name.clone(),
                        span: expr.span,
                    })
            }
            ExprKind::Binary { op, left, right } => self.eval_binary(*op, left, right, expr.span),
            ExprKind::Unary { op, expr: inner } => {
                let val = self.eval_expr(inner)?;
                match op {
                    UnOp::Neg => match val {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(EvalError::TypeError {
                            message: format!("Cannot negate type `{}`", other.type_name()),
                            span: expr.span,
                        }),
                    },
                    UnOp::Not => Ok(Value::Bool(!val.is_truthy())),
                }
            }
            ExprKind::Call { callee, args } => {
                // If callee is a method call on an instance `obj.method(...)`
                if let ExprKind::MemberAccess { object, member, .. } = &callee.kind {
                    let obj_val = self.eval_expr(object)?;
                    if let Value::Instance(ref inst) = obj_val {
                        if let Some(method_fn) = inst.methods.get(member) {
                            let mut evaluated_args = vec![obj_val.clone()];
                            for arg in args {
                                evaluated_args.push(self.eval_expr(&arg.value)?);
                            }
                            let fn_val = Value::Function(method_fn.clone());
                            return self.call_value(&fn_val, &evaluated_args, expr.span);
                        }
                    }
                }

                let callee_val = self.eval_expr(callee)?;
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(&arg.value)?);
                }
                self.call_value(&callee_val, &evaluated_args, expr.span)
            }
            ExprKind::MemberAccess {
                object,
                member,
                is_safe,
            } => {
                let obj_val = self.eval_expr(object)?;
                if *is_safe && matches!(obj_val, Value::None) {
                    return Ok(Value::None);
                }

                match obj_val {
                    Value::Instance(inst) => {
                        if let Some(val) = inst.fields.borrow().get(member) {
                            Ok(val.clone())
                        } else if let Some(m) = inst.methods.get(member) {
                            Ok(Value::Function(m.clone()))
                        } else {
                            Err(EvalError::FieldNotFound {
                                field: member.clone(),
                                type_name: inst.struct_name,
                                span: expr.span,
                            })
                        }
                    }
                    Value::Map(m) => {
                        if let Some(val) = m.borrow().get(member) {
                            Ok(val.clone())
                        } else if *is_safe {
                            Ok(Value::None)
                        } else {
                            Err(EvalError::KeyNotFound {
                                key: member.clone(),
                                span: expr.span,
                            })
                        }
                    }
                    other => {
                        // Built-in methods like .len
                        if member == "len" {
                            match &other {
                                Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                                Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
                                Value::Map(m) => Ok(Value::Int(m.borrow().len() as i64)),
                                _ => Err(EvalError::FieldNotFound {
                                    field: member.clone(),
                                    type_name: other.type_name().to_string(),
                                    span: expr.span,
                                }),
                            }
                        } else {
                            Err(EvalError::FieldNotFound {
                                field: member.clone(),
                                type_name: other.type_name().to_string(),
                                span: expr.span,
                            })
                        }
                    }
                }
            }
            ExprKind::Index { object, index } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;

                match (obj_val, idx_val) {
                    (Value::List(l), Value::Int(i)) => {
                        let list = l.borrow();
                        let actual_idx = if i < 0 {
                            list.len() as i64 + i
                        } else {
                            i
                        };
                        if actual_idx < 0 || actual_idx as usize >= list.len() {
                            return Err(EvalError::IndexOutOfBounds {
                                index: i,
                                len: list.len(),
                                span: expr.span,
                            });
                        }
                        Ok(list[actual_idx as usize].clone())
                    }
                    (Value::String(s), Value::Int(i)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let actual_idx = if i < 0 {
                            chars.len() as i64 + i
                        } else {
                            i
                        };
                        if actual_idx < 0 || actual_idx as usize >= chars.len() {
                            return Err(EvalError::IndexOutOfBounds {
                                index: i,
                                len: chars.len(),
                                span: expr.span,
                            });
                        }
                        Ok(Value::String(chars[actual_idx as usize].to_string()))
                    }
                    (Value::Map(m), Value::String(k)) => {
                        let map = m.borrow();
                        map.get(&k).cloned().ok_or_else(|| EvalError::KeyNotFound {
                            key: k,
                            span: expr.span,
                        })
                    }
                    (other_obj, other_idx) => Err(EvalError::TypeError {
                        message: format!(
                            "Cannot index `{}` with `{}`",
                            other_obj.type_name(),
                            other_idx.type_name()
                        ),
                        span: expr.span,
                    }),
                }
            }
            ExprKind::List(items) => {
                let mut list = Vec::new();
                for item in items {
                    list.push(self.eval_expr(item)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(list))))
            }
            ExprKind::Map(entries) => {
                let mut map = HashMap::new();
                for (k_expr, v_expr) in entries {
                    let k = self.eval_expr(k_expr)?;
                    let v = self.eval_expr(v_expr)?;
                    map.insert(k.to_string(), v);
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }
            ExprKind::Lambda { params, body } => {
                let fn_val = FnValue {
                    name: "<lambda>".to_string(),
                    params: params.clone(),
                    body: vec![Stmt::new(StmtKind::Return(Some(*body.clone())), expr.span)],
                    env: Rc::clone(&self.env),
                };
                Ok(Value::Function(fn_val))
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    self.eval_expr(then_branch)
                } else if let Some(else_b) = else_branch {
                    self.eval_expr(else_b)
                } else {
                    Ok(Value::None)
                }
            }
            ExprKind::Try(inner) => {
                let val = self.eval_expr(inner)?;
                if matches!(val, Value::None) {
                    return Ok(Value::None);
                }
                Ok(val)
            }
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let s_val = self.eval_expr(start)?;
                let e_val = self.eval_expr(end)?;
                match (s_val, e_val) {
                    (Value::Int(s), Value::Int(e)) => {
                        let end_adj = if *inclusive { e + 1 } else { e };
                        let list: Vec<Value> = (s..end_adj).map(Value::Int).collect();
                        Ok(Value::List(Rc::new(RefCell::new(list))))
                    }
                    _ => Err(EvalError::TypeError {
                        message: "Range operands must be integers".to_string(),
                        span: expr.span,
                    }),
                }
            }
            ExprKind::Block(stmts) => match self.eval_block(stmts)? {
                ControlFlow::Normal(v) | ControlFlow::Return(v) => Ok(v),
                _ => Ok(Value::None),
            },
        }
    }

    fn eval_binary(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Result<Value, EvalError> {
        // Short-circuit logical ops
        if op == BinOp::And {
            let l_val = self.eval_expr(left)?;
            if !l_val.is_truthy() {
                return Ok(l_val);
            }
            return self.eval_expr(right);
        }
        if op == BinOp::Or {
            let l_val = self.eval_expr(left)?;
            if l_val.is_truthy() {
                return Ok(l_val);
            }
            return self.eval_expr(right);
        }
        if op == BinOp::NullCoalesce {
            let l_val = self.eval_expr(left)?;
            if !matches!(l_val, Value::None) {
                return Ok(l_val);
            }
            return self.eval_expr(right);
        }
        if op == BinOp::Pipeline {
            // `data |> fn_call(arg2)` -> `fn_call(data, arg2)`
            let l_val = self.eval_expr(left)?;
            if let ExprKind::Call { callee, args } = &right.kind {
                let callee_val = self.eval_expr(callee)?;
                let mut evaluated_args = vec![l_val];
                for arg in args {
                    evaluated_args.push(self.eval_expr(&arg.value)?);
                }
                return self.call_value(&callee_val, &evaluated_args, span);
            } else {
                let r_val = self.eval_expr(right)?;
                return self.call_value(&r_val, &[l_val], span);
            }
        }

        let l_val = self.eval_expr(left)?;
        let r_val = self.eval_expr(right)?;

        match op {
            BinOp::Add => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                (Value::String(a), other) => Ok(Value::String(format!("{}{}", a, other))),
                (other, Value::String(b)) => Ok(Value::String(format!("{}{}", other, b))),
                (Value::List(a), Value::List(b)) => {
                    let mut combined = a.borrow().clone();
                    combined.extend(b.borrow().clone());
                    Ok(Value::List(Rc::new(RefCell::new(combined))))
                }
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot add `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Sub => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot subtract `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Mul => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
                (Value::String(s), Value::Int(n)) | (Value::Int(n), Value::String(s)) => {
                    if n < 0 {
                        Ok(Value::String("".to_string()))
                    } else {
                        Ok(Value::String(s.repeat(n as usize)))
                    }
                }
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot multiply `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Div => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    if a % b == 0 {
                        Ok(Value::Int(a / b))
                    } else {
                        Ok(Value::Float(a as f64 / b as f64))
                    }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a / b))
                }
                (Value::Int(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a as f64 / b))
                }
                (Value::Float(a), Value::Int(b)) => {
                    if b == 0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a / b as f64))
                }
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot divide `{}` by `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Mod => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Int(a % b))
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a % b))
                }
                (Value::Int(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float((a as f64) % b))
                }
                (Value::Float(a), Value::Int(b)) => {
                    if b == 0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a % (b as f64)))
                }
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot modulo `{}` by `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Pow => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => {
                    if b >= 0 {
                        Ok(Value::Int(a.pow(b as u32)))
                    } else {
                        Ok(Value::Float((a as f64).powf(b as f64)))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float((a as f64).powf(b))),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powi(b as i32))),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot power `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Eq => Ok(Value::Bool(l_val.is_equal(&r_val))),
            BinOp::NotEq => Ok(Value::Bool(!l_val.is_equal(&r_val))),
            BinOp::Lt => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((a as f64) < b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(a < (b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot compare `<` between `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::LtEq => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((a as f64) <= b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(a <= (b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a <= b)),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot compare `<=` between `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::Gt => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((a as f64) > b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(a > (b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a > b)),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot compare `>` between `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinOp::GtEq => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((a as f64) >= b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(a >= (b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a >= b)),
                (a, b) => Err(EvalError::TypeError {
                    message: format!("Cannot compare `>=` between `{}` and `{}`", a.type_name(), b.type_name()),
                    span,
                }),
            },
            _ => unreachable!(),
        }
    }

    pub fn call_value(
        &mut self,
        callee: &Value,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        match callee {
            Value::Builtin(_, f) => f(args, span, self),
            Value::Function(func) => {
                if args.len() > func.params.len() {
                    return Err(EvalError::ArgCountMismatch {
                        expected: func.params.len(),
                        got: args.len(),
                        span,
                    });
                }

                let mut call_env = Environment::with_parent(Rc::clone(&func.env));
                for (i, param) in func.params.iter().enumerate() {
                    if i < args.len() {
                        call_env.define_mut(param.name.clone(), args[i].clone());
                    } else if let Some(ref def_expr) = param.default_val {
                        let def_val = self.eval_expr(def_expr)?;
                        call_env.define_mut(param.name.clone(), def_val);
                    } else {
                        return Err(EvalError::ArgCountMismatch {
                            expected: func.params.len(),
                            got: args.len(),
                            span,
                        });
                    }
                }

                let prev_env = Rc::clone(&self.env);
                self.env = Rc::new(RefCell::new(call_env));

                let mut return_val = Value::None;
                for stmt in &func.body {
                    match self.eval_stmt(stmt)? {
                        ControlFlow::Normal(_) => {}
                        ControlFlow::Return(v) => {
                            return_val = v;
                            break;
                        }
                        ControlFlow::Break => {
                            self.env = prev_env;
                            return Err(EvalError::Generic {
                                message: "Break outside of loop".to_string(),
                                span: stmt.span,
                            });
                        }
                        ControlFlow::Continue => {
                            self.env = prev_env;
                            return Err(EvalError::Generic {
                                message: "Continue outside of loop".to_string(),
                                span: stmt.span,
                            });
                        }
                    }
                }

                self.env = prev_env;
                Ok(return_val)
            }
            Value::Struct(def) => {
                let mut field_values = HashMap::new();
                for (i, f) in def.fields.iter().enumerate() {
                    if i < args.len() {
                        field_values.insert(f.name.clone(), args[i].clone());
                    } else if let Some(ref def_expr) = f.default_val {
                        let def_val = self.eval_expr(def_expr)?;
                        field_values.insert(f.name.clone(), def_val);
                    } else {
                        field_values.insert(f.name.clone(), Value::None);
                    }
                }

                let inst = StructInstance {
                    struct_name: def.name.clone(),
                    fields: Rc::new(RefCell::new(field_values)),
                    methods: def.methods.clone(),
                };
                Ok(Value::Instance(inst))
            }
            other => Err(EvalError::NotCallable {
                type_name: other.type_name().to_string(),
                span,
            }),
        }
    }

    fn handle_assign(
        &mut self,
        target: &AssignTarget,
        op: Option<BinOp>,
        rhs: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        match target {
            AssignTarget::Variable(name) => {
                let final_val = if let Some(bin_op) = op {
                    let curr_val = self
                        .env
                        .borrow()
                        .get(name)
                        .ok_or_else(|| EvalError::UndefinedVariable {
                            name: name.clone(),
                            span,
                        })?;
                    self.eval_binary_values(bin_op, curr_val, rhs, span)?
                } else {
                    rhs
                };
                self.env.borrow_mut().assign(name, final_val, span)?;
            }
            AssignTarget::Member { object, member } => {
                let obj_val = self.eval_expr(object)?;
                match obj_val {
                    Value::Instance(inst) => {
                        let final_val = if let Some(bin_op) = op {
                            let curr_val = inst
                                .fields
                                .borrow()
                                .get(member)
                                .cloned()
                                .unwrap_or(Value::None);
                            self.eval_binary_values(bin_op, curr_val, rhs, span)?
                        } else {
                            rhs
                        };
                        inst.fields.borrow_mut().insert(member.clone(), final_val);
                    }
                    Value::Map(m) => {
                        let final_val = if let Some(bin_op) = op {
                            let curr_val =
                                m.borrow().get(member).cloned().unwrap_or(Value::None);
                            self.eval_binary_values(bin_op, curr_val, rhs, span)?
                        } else {
                            rhs
                        };
                        m.borrow_mut().insert(member.clone(), final_val);
                    }
                    other => {
                        return Err(EvalError::TypeError {
                            message: format!(
                                "Cannot assign field `{}` on type `{}`",
                                member,
                                other.type_name()
                            ),
                            span,
                        })
                    }
                }
            }
            AssignTarget::Index { object, index } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;
                match (obj_val, idx_val) {
                    (Value::List(l), Value::Int(i)) => {
                        let mut list = l.borrow_mut();
                        let actual_idx = if i < 0 {
                            list.len() as i64 + i
                        } else {
                            i
                        };
                        if actual_idx < 0 || actual_idx as usize >= list.len() {
                            return Err(EvalError::IndexOutOfBounds {
                                index: i,
                                len: list.len(),
                                span,
                            });
                        }
                        let final_val = if let Some(bin_op) = op {
                            let curr_val = list[actual_idx as usize].clone();
                            self.eval_binary_values(bin_op, curr_val, rhs, span)?
                        } else {
                            rhs
                        };
                        list[actual_idx as usize] = final_val;
                    }
                    (Value::Map(m), Value::String(k)) => {
                        let mut map = m.borrow_mut();
                        let final_val = if let Some(bin_op) = op {
                            let curr_val = map.get(&k).cloned().unwrap_or(Value::None);
                            self.eval_binary_values(bin_op, curr_val, rhs, span)?
                        } else {
                            rhs
                        };
                        map.insert(k, final_val);
                    }
                    (other_obj, other_idx) => {
                        return Err(EvalError::TypeError {
                            message: format!(
                                "Cannot index-assign `{}` with `{}`",
                                other_obj.type_name(),
                                other_idx.type_name()
                            ),
                            span,
                        })
                    }
                }
            }
        }
        Ok(())
    }

    fn eval_binary_values(
        &mut self,
        op: BinOp,
        l_val: Value,
        r_val: Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        match op {
            BinOp::Add => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                (Value::String(a), other) => Ok(Value::String(format!("{}{}", a, other))),
                (other, Value::String(b)) => Ok(Value::String(format!("{}{}", other, b))),
                _ => Err(EvalError::TypeError {
                    message: "Invalid types for +=".to_string(),
                    span,
                }),
            },
            BinOp::Sub => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
                _ => Err(EvalError::TypeError {
                    message: "Invalid types for -=".to_string(),
                    span,
                }),
            },
            BinOp::Mul => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
                _ => Err(EvalError::TypeError {
                    message: "Invalid types for *=".to_string(),
                    span,
                }),
            },
            BinOp::Div => match (l_val, r_val) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Int(a / b))
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(EvalError::DivisionByZero { span });
                    }
                    Ok(Value::Float(a / b))
                }
                _ => Err(EvalError::TypeError {
                    message: "Invalid types for /=".to_string(),
                    span,
                }),
            },
            _ => Err(EvalError::TypeError {
                message: "Unsupported assignment operator".to_string(),
                span,
            }),
        }
    }

    fn interpolate_string(&mut self, s: &str, span: Span) -> Result<Value, EvalError> {
        if !s.contains('{') {
            return Ok(Value::String(s.to_string()));
        }

        let mut out = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    // Escaped {{
                    chars.next();
                    out.push('{');
                    continue;
                }

                // Extract expression inside {...}
                let mut expr_str = String::new();
                let mut depth = 1;
                while let Some(ch) = chars.next() {
                    if ch == '{' {
                        depth += 1;
                        expr_str.push(ch);
                    } else if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(ch);
                    } else {
                        expr_str.push(ch);
                    }
                }

                // Parse and evaluate sub-expression
                let mut lexer = Lexer::new(&expr_str);
                if let Ok(tokens) = lexer.tokenize_all() {
                    let mut parser = Parser::new(tokens);
                    if let Ok(sub_expr) = parser.parse_expr() {
                        if let Ok(eval_res) = self.eval_expr(&sub_expr) {
                            out.push_str(&eval_res.to_string());
                            continue;
                        }
                    }
                }

                // Fallback: preserve literal pattern (e.g. for URL patterns "/users/{id}")
                out.push('{');
                out.push_str(&expr_str);
                out.push('}');
            } else if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                out.push('}');
            } else {
                out.push(c);
            }
        }

        Ok(Value::String(out))
    }
}
