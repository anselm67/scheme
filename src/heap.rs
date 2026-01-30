use std::{collections::HashMap};

use crate::{interp::Interp, types::{GcId, SchemeError, SchemeObject, Value}};

pub type PrimitiveFn = fn(&Interp, &[Value]) -> Result<Value, SchemeError>;


enum HeapObject {
    List(Vec<Value>),
    Symbol(String),
    String(String),
    Primitive(PrimitiveFn),
    // Other heap-allocated object types can be added here
}

pub struct Heap {
    objects: Vec<HeapObject>,
    symbols: HashMap<String, GcId>,
}

impl Heap {
    
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            symbols: HashMap::new(),
        }
    }

    fn get(&self, id: GcId) -> &HeapObject {
        &self.objects[id]
    }

    pub fn intern_symbol(&mut self, name: &str) -> Value {
        if let Some(&id) = self.symbols.get(name) {
            return Value::Object(id);
        }
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::Symbol(name.to_string()));
        self.symbols.insert(name.to_string(), id);
        Value::Object(id)
    }
    
    pub fn alloc_list(&mut self, elements: Vec<Value>) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::List(elements));
        Value::Object(id)
    }

    pub fn alloc_string(&mut self, s: String) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::String(s));
        Value::Object(id)
    }

    pub fn alloc_primitive(&mut self, func: PrimitiveFn) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::Primitive(func));
        Value::Object(id)
    }

}
pub trait Apply {
    fn apply(&self, interp: &Interp, args: Vec<Value>) -> Result<Value, SchemeError>;
}

impl Apply for Value {
    fn apply(&self, interp: &Interp, args: Vec<Value>) -> Result<Value, SchemeError> {
        match self {
            Value::Object(id) => {
                let obj = interp.heap.get(*id);
                match obj {
                    HeapObject::Primitive(pr) => pr(interp, &args),
                    _ => Err(SchemeError::TypeError("Attempted to apply a non-primitive object".to_string())),
                }
            },
            _ => Err(SchemeError::TypeError("Attempted to apply a non-object value".to_string())),
        }
    }
}


impl SchemeObject for GcId {
    fn eval(&self, interp: &Interp) -> Result<Value, SchemeError> {
        let id = *self;
        let obj = interp.heap.get(id);
        match obj {
            HeapObject::List(elements) => {
                match elements.as_slice() {
                    [] => Ok(Value::Nil),
                    [func, rest @ ..] => {
                        let args = rest.iter()
                            .map(|arg| arg.eval(interp))
                            .collect::<Result<Vec<Value>, SchemeError>>()?;
                        func.eval(interp)?.apply(&interp, args)
                    }    
                }
            }
            HeapObject::Symbol(name) => {
                interp.env.lookup(id)
                    .ok_or_else(|| SchemeError::EvalError(format!("Unbound symbol with id {}", name)))
            }
            _ => Ok(Value::Object(id))
        }
    }

    fn display(&self, interp: &Interp) -> String {
        let id = *self;
        let obj = interp.heap.get(id);
        match obj {
            HeapObject::List(elements) => {
                let elems_str: Vec<String> = elements.iter().map(|e| e.display(interp)).collect();
                format!("({})", elems_str.join(" "))
            },
            HeapObject::Symbol(s) => format!("{}", s),
            HeapObject::Primitive(pr) => format!("<primitive {:p}>", pr),
            HeapObject::String(s) => format!("\"{}\"", s),
        }
    }
}