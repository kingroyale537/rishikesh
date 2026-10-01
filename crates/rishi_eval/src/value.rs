use rishi_ast::*;
use rishi_span::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum EvalError {
    #[error("Type error: {message}")]
    TypeError { message: String, span: Span },
    #[error("Undefined variable `{name}`")]
    UndefinedVariable { name: String, span: Span },
    #[error("Cannot reassign to immutable variable `{name}` (declared with `let`). Use `mut` to allow reassignment.")]
    ImmutableAssignment { name: String, span: Span },
    #[error("Division by zero")]
    DivisionByZero { span: Span },
    #[error("Index out of bounds: index {index}, length {len}")]
    IndexOutOfBounds { index: i64, len: usize, span: Span },
    #[error("Key not found: `{key}`")]
    KeyNotFound { key: String, span: Span },
    #[error("Field not found: `{field}` on object of type `{type_name}`")]
    FieldNotFound { field: String, type_name: String, span: Span },
    #[error("Assertion failed: {message}")]
    AssertionFailed { message: String, span: Span },
    #[error("Argument count mismatch: expected {expected}, got {got}")]
    ArgCountMismatch { expected: usize, got: usize, span: Span },
    #[error("Not callable: `{type_name}` is not a function")]
    NotCallable { type_name: String, span: Span },
    #[error("Runtime error: {message}")]
    Generic { message: String, span: Span },
}

pub type BuiltinFn = fn(&[Value], Span, &mut crate::evaluator::Evaluator) -> Result<Value, EvalError>;

#[derive(Clone)]
pub struct FnValue {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub env: Rc<RefCell<crate::env::Environment>>,
}

impl fmt::Debug for FnValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}>", self.name)
    }
}

impl PartialEq for FnValue {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub methods: HashMap<String, FnValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructInstance {
    pub struct_name: String,
    pub fields: Rc<RefCell<HashMap<String, Value>>>,
    pub methods: HashMap<String, FnValue>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Struct(StructDef),
    Instance(StructInstance),
    Function(FnValue),
    Builtin(String, BuiltinFn),
    None,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::List(a), Value::List(b)) => *a.borrow() == *b.borrow(),
            (Value::Map(a), Value::Map(b)) => *a.borrow() == *b.borrow(),
            (Value::Struct(a), Value::Struct(b)) => a.name == b.name,
            (Value::Instance(a), Value::Instance(b)) => a.struct_name == b.struct_name && *a.fields.borrow() == *b.fields.borrow(),
            (Value::Function(a), Value::Function(b)) => a.name == b.name,
            (Value::Builtin(a, _), Value::Builtin(b, _)) => a == b,
            (Value::None, Value::None) => true,
            _ => false,
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Struct(_) => "StructDef",
            Value::Instance(_) => "Instance",
            Value::Function(_) => "Function",
            Value::Builtin(_, _) => "BuiltinFunction",
            Value::None => "None",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            Value::None => false,
            _ => true,
        }
    }

    pub fn is_equal(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::List(a), Value::List(b)) => *a.borrow() == *b.borrow(),
            (Value::Map(a), Value::Map(b)) => *a.borrow() == *b.borrow(),
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => {
                if fl.fract() == 0.0 {
                    write!(f, "{:.1}", fl)
                } else {
                    write!(f, "{}", fl)
                }
            }
            Value::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Value::String(s) => write!(f, "{}", s),
            Value::List(items) => {
                let items = items.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Value::String(s) = item {
                        write!(f, "\"{}\"", s)?;
                    } else {
                        write!(f, "{}", item)?;
                    }
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                let entries = entries.borrow();
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in entries.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "\"{}\": ", k)?;
                    if let Value::String(s) = v {
                        write!(f, "\"{}\"", s)?;
                    } else {
                        write!(f, "{}", v)?;
                    }
                }
                write!(f, "}}")
            }
            Value::Struct(s) => write!(f, "<struct {}>", s.name),
            Value::Instance(inst) => {
                let fields = inst.fields.borrow();
                write!(f, "{}(", inst.struct_name)?;
                let mut first = true;
                for (k, v) in fields.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}={}", k, v)?;
                }
                write!(f, ")")
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::Builtin(name, _) => write!(f, "<builtin fn {}>", name),
            Value::None => write!(f, "None"),
        }
    }
}
