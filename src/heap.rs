use std::{collections::HashMap};

use crate::{interp::{Interp}, types::{GcId, SchemeError, SchemeObject, Value}};

pub type PrimitiveFn = fn(&Interp, &[Value]) -> Result<Value, SchemeError>;


enum HeapObject {
    List(Vec<Value>),
    Symbol(String),
    String(String),
    Primitive(PrimitiveFn),
    // Other heap-allocated object types can be added here
}

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Keyword {
    If = 0,
    Define = 1,
    Lambda = 2,
    Quote = 3,
    True = 4,
    False = 5,
}

impl Keyword {

    fn from_id(id: GcId) -> Option<Keyword> {
        match id {
            0 => Some(Keyword::If),
            1 => Some(Keyword::Define),
            2 => Some(Keyword::Lambda),
            3 => Some(Keyword::Quote),
            4 => Some(Keyword::True),
            5 => Some(Keyword::False),
            _ => None,
        }
    }

    fn eval(interp: &Interp, keyword: Keyword, args: &[Value]) -> Result<Value, SchemeError> {
        match keyword {
            Keyword::If => {
                if args.len() != 3 {
                    return Err(SchemeError::EvalError("if expects exactly 3 arguments".to_string()));
                }
                let condition = args[0].eval(interp)?;
                match condition {
                    Value::Boolean(true) => args[1].eval(interp),
                    Value::Boolean(false) => args[2].eval(interp),
                    _ => Err(SchemeError::TypeError("if condition must evaluate to a boolean".to_string())),
                }
            }
            _ => {
                return Err(SchemeError::EvalError("not implemented".to_string()));
            }
        }
    }
}


pub struct Heap {
    objects: Vec<HeapObject>,
    symbols: HashMap<String, GcId>,
}

impl Heap {
    
    pub fn new() -> Self {
        let mut heap = Self {
            objects: Vec::new(),
            symbols: HashMap::new(),
        };
        // Pre-intern keywords
        heap.intern_special_keywwords();
        heap
    }

    fn intern_special_keywwords(&mut self) {
        let if_id =self.intern_symbol_to_gcid("if");
        assert!(if_id == Keyword::If as usize, "Keyword 'if' should have GcId 0");
        let define_id = self.intern_symbol_to_gcid("define");
        assert!(define_id == Keyword::Define as usize, "Keyword 'define' should have GcId 1");
        let lambda_id = self.intern_symbol_to_gcid("lambda");
        assert!(lambda_id == Keyword::Lambda as usize, "Keyword 'lambda' should have GcId 2");
        let quote_id = self.intern_symbol_to_gcid("quote");
        assert!(quote_id == Keyword::Quote as usize, "Keyword 'quote' should have GcId 3");
        let true_id = self.intern_symbol_to_gcid("#t");
        assert!(true_id == Keyword::True as usize, "Keyword '#t' should have GcId 4");
        let false_id = self.intern_symbol_to_gcid("#f");
        assert!(false_id == Keyword::False as usize, "Keyword '#f' should have GcId 5");
    }

    fn get(&self, id: GcId) -> &HeapObject {
        &self.objects[id]
    }

    fn intern_symbol_to_gcid(&mut self, name: &str) -> GcId {
        if let Some(&id) = self.symbols.get(name) {
            return id;
        } else {
            let id: GcId = self.objects.len();
            self.objects.push(HeapObject::Symbol(name.to_string()));
            self.symbols.insert(name.to_string(), id);
            id
        }
    }
    
    pub fn intern_symbol(&mut self, name: &str) -> Value {
        Value::Object(self.intern_symbol_to_gcid(name))
    }

    pub fn alloc_list(&mut self, elements: Vec<Value>) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::List(elements));
        Value::Object(id)
    }

    pub fn alloc_string(&mut self, s: impl Into<String>) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::String(s.into()));
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
                        if let Value::Object(func_id) = func 
                            && let Some(keyword) = Keyword::from_id(*func_id) {
                                // Special form handling
                                Keyword::eval(interp, keyword, rest)
                        } else {
                        // Fallback if not a pecial form.
                            let args = rest.iter()
                                .map(|arg| arg.eval(interp))
                                .collect::<Result<Vec<Value>, SchemeError>>()?;
                            func.eval(interp)?.apply(&interp, args)
                        }
                    }    
                }
            }
            HeapObject::Symbol(name) => {
                interp.env.lookup(id)
                    .ok_or_else(|| SchemeError::UnboundVariable(format!("Unbound symbol with id {}", name)))
            }
            _ => Ok(Value::Object(id))
        }
    }

    fn is_false(&self) -> bool {
        return *self == Keyword::False as usize;
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