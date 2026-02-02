use std::cell::RefCell;
use std::collections::HashMap;
use std::process;
use std::rc::Rc;

use crate::{all_numbers, extract_args, heap};
use crate::types::{Number, SchemeError, SchemeObject, Value};

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

    pub fn define_primitive(&self, name: &str, func: heap::PrimitiveFn) {
        let prim = self.heap.borrow_mut().alloc_primitive(func);
        self.define(name, prim);
    }

    fn init(&self) {
        self.define("#t", Value::Boolean(true));
        self.define("#f", Value::Boolean(false));
        // Initialize primitive functions
        self.define_primitive("+", primitive_add);
        self.define_primitive("*", primitive_mul);
        self.define_primitive("=", primitive_number_eq);
        self.define_primitive("quit", primitive_quit);
        self.define_primitive("exit", primitive_quit);
    }

    pub fn lookup(&self, name: &str) -> Value {
        self.heap.borrow_mut().intern_symbol(name)
    }

    pub fn eval(&self, obj: Value)  -> Result<Value, SchemeError> {
        obj.eval(self, &self.env) 
    }

    pub fn display(&self, obj: Value) {
        let output = obj.display(&self);
        println!("{}", output);
    }
}

fn primitive_add(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_numbers!(args);
    let sum = nums.into_iter()
        .fold(Number::Int(0), |acc, n| acc  + *n);
    Ok(Value::Number(sum))
}

fn primitive_mul(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_numbers!(args);
    let mul = nums.into_iter()
        .fold(Number::Int(1), |acc, n| acc * *n);
    Ok(Value::Number(mul))
}

fn primitive_quit(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 1, exit_code: Number);
    match i32::try_from(*exit_code) {
        Ok(code) => process::exit(code),
        Err(_) => Err(SchemeError::OverflowError(format!(
            "Overflow while converting {} to i32", exit_code)
        ))
    }

}

fn primitive_number_eq(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    return Ok(Value::Boolean(a == b))
}
