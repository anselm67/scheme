use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{interp::Interp, markset::MarkSet, types::{GcId, SchemeError, SchemeObject, Value}};


pub struct Env {
    pub macros: HashMap<GcId, Value>,
    pub bindings: HashMap<GcId, Value>,
    pub parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn extend(parent: Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
        Rc::new(RefCell::new(Env {
            macros: HashMap::new(),
            bindings: HashMap::new(),
            parent: Some(parent),
        }))
    }

    pub fn define(&mut self, key: GcId, value: Value) {
        self.bindings.insert(key, value);
    }

    pub fn define_syntax(&mut self, key: GcId, value: Value) {
        self.macros.insert(key, value);
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

    pub fn mark(&self, interp: &Interp, marks: &mut MarkSet) {
        // TODO If this env was already marked, skip this.
        loop {
            // Marks the macros and their definitions.
            for (id, value) in self.macros.iter() {
                id.mark(interp, marks);
                value.mark(interp, marks);
            }
            // Marks the symbols and their values.
            for (id, value) in self.bindings.iter() {
                id.mark(interp, marks);
                value.mark(interp, marks);
            }
            // Marks the parent environment.
            match &self.parent {
                Some(parent_env) => {
                    let outer = parent_env.borrow();
                    outer.mark(interp, marks);
                },
                None => return,
            }
        }
    }
}