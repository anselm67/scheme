use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::types::{GcId, SchemeError, Value};


pub struct Env {
    pub bindings: HashMap<GcId, Value>,
    pub parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn extend(parent: Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
        Rc::new(RefCell::new(Env {
            bindings: HashMap::new(),
            parent: Some(parent),
        }))
    }

    pub fn define(&mut self, key: GcId, value: Value) {
        self.bindings.insert(key, value);
    }

    pub fn set_bang(&mut self, key: GcId, value: Value) -> Result<(), SchemeError> {
        if self.bindings.contains_key(&key) {
            self.bindings.insert(key, value);
            Ok(())
        } else {
            match &self.parent {
                Some(parent_env) => parent_env.borrow_mut().set_bang(key, value),
                None => Err(SchemeError::UnboundVariable(format!("Unbound variable with GcId {}", key))),
            }
        }
    }

    pub fn lookup(&self, key: GcId) -> Option<Value> {
        if let Some(value) = self.bindings.get(&key) {
            Some(*value)
        } else {
            match &self.parent {
                Some(parent_env) => parent_env.borrow().lookup(key),
                None => None,
            }
        }
    }
}