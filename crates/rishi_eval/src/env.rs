use crate::value::{EvalError, Value};
use rishi_span::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Binding {
    pub value: Value,
    pub is_mutable: bool,
}

#[derive(Clone, Debug)]
pub struct Environment {
    parent: Option<Rc<RefCell<Environment>>>,
    bindings: HashMap<String, Binding>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            parent: None,
            bindings: HashMap::new(),
        }
    }

    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Self {
            parent: Some(parent),
            bindings: HashMap::new(),
        }
    }

    pub fn define_let(&mut self, name: String, value: Value) {
        self.bindings.insert(
            name,
            Binding {
                value,
                is_mutable: false,
            },
        );
    }

    pub fn define_mut(&mut self, name: String, value: Value) {
        self.bindings.insert(
            name,
            Binding {
                value,
                is_mutable: true,
            },
        );
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(b) = self.bindings.get(name) {
            return Some(b.value.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    pub fn assign(&mut self, name: &str, new_val: Value, span: Span) -> Result<(), EvalError> {
        if let Some(b) = self.bindings.get_mut(name) {
            if !b.is_mutable {
                return Err(EvalError::ImmutableAssignment {
                    name: name.to_string(),
                    span,
                });
            }
            b.value = new_val;
            return Ok(());
        }

        if let Some(parent) = &self.parent {
            return parent.borrow_mut().assign(name, new_val, span);
        }

        Err(EvalError::UndefinedVariable {
            name: name.to_string(),
            span,
        })
    }
}
