use crate::env;
use crate::heap;
use crate::types::{SchemeObject, SchemeError, Value};

pub struct Interp {
    pub heap: heap::Heap,
    pub env: env::Env,
}

impl Interp {
    pub fn new() -> Self {
        let mut interp = Self {
            heap: heap::Heap::new(),
            env: env::Env::new(),
        };
        interp.init();
        interp
    }

    pub fn define(&mut self, name: &str, value: Value) {
        let symbol = self.heap.intern_symbol(name);
        if let Value::Object(id) = symbol {
            self.env.define(id, value);
        }
    }

    pub fn define_primitives(&mut self, name: &str, func: heap::PrimitiveFn) {
        let prim = self.heap.alloc_primitive(func);
        self.define(name, prim);
    }

    fn init(&mut self) {
        // Initialize primitive functions
        self.define_primitives("+", primitive_add);
        self.define_primitives("*", primitive_mul);
    }

    pub fn lookup(&mut self, name: &str) -> Value {
        self.heap.intern_symbol(name)
    }

    pub fn eval(&mut self, obj: &Value)  -> Result<Value, SchemeError> {
        obj.eval(&self) 
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
