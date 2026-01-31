use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::heap;
use crate::types::{SchemeObject, SchemeError, Value};

pub struct Interp {
    pub heap: RefCell<heap::Heap>,
    pub env: Rc<RefCell<crate::env::Env>>,
}

impl Interp {
    pub fn new() -> Self {
        let global_env = crate::env::Env {
            bindings: HashMap::new(),
            parent: None,
        };
        let env_handle = Rc::new(RefCell::new(global_env));
        let heap_handlee = RefCell::new(heap::Heap::new());
        let interp = Self {
            heap: heap_handlee,
            env: env_handle,
        };
        interp.init();
        interp
    }

    pub fn define(&self, name: &str, value: Value) {
        let symbol = self.heap.borrow_mut().intern_symbol(name);
        if let Value::Object(id) = symbol {
            self.env.borrow_mut().define(id, value);
        }
    }

    pub fn define_primitives(&self, name: &str, func: heap::PrimitiveFn) {
        let prim = self.heap.borrow_mut().alloc_primitive(func);
        self.define(name, prim);
    }

    fn init(&self) {
        self.define("#t", Value::Boolean(true));
        self.define("#f", Value::Boolean(false));
        // Initialize primitive functions
        self.define_primitives("+", primitive_add);
        self.define_primitives("*", primitive_mul);
    }

    pub fn lookup(&self, name: &str) -> Value {
        self.heap.borrow_mut().intern_symbol(name)
    }

    pub fn eval(&self, obj: &Value)  -> Result<Value, SchemeError> {
        obj.eval(self) 
    }

    pub fn display(&self, obj: &Value) {
        let output = obj.display(&self);
        println!("{}", output);
    }
}

fn primitive_add(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let sum = args.iter().try_fold(0, |acc, val| {
        match val {
            Value::Integer(i) => Ok(acc + i),
            _ => Err(SchemeError::TypeError("Expected integer".to_string())),
        }
    })?;
    Ok(Value::Integer(sum))
}

fn primitive_mul(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let sum = args.iter().try_fold(1, |acc, val| {
        match val {
            Value::Integer(i) => Ok(acc * i),
            _ => Err(SchemeError::TypeError("Expected integer".to_string())),
        }
    })?;
    Ok(Value::Integer(sum))
}
