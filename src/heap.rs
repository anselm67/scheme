use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    env::Env, interp::Interp, types::{GcId, SchemeError, SchemeObject, Value}
};

pub type PrimitiveFn = fn(&Interp, &[Value]) -> Result<Value, SchemeError>;


#[derive(Clone)]
pub struct Closure {
    params: Box<[GcId]>,
    body: Box<[Value]>,
    env: Rc<RefCell<Env>>,
}

#[derive(Clone)]
pub enum HeapObject {
    FreeSlot(GcId),
    Pair(Value, Value),
    List(Vec<Value>),
    Symbol(String),
    String(String),
    Primitive(PrimitiveFn),
    Closure(Box<Closure>),
    // Other heap-allocated object types can be added here
}

impl HeapObject {

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::FreeSlot(_) => "FreeSlot",
            Self::Pair(..) => "Pair",
            Self::List(_) => "List",
            Self::Symbol(_) => "Symbol",
            Self::String(_) => "String",
            Self::Primitive(_) => "Primitive",
            Self::Closure(_) => "Closure",
        }
    }
}

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Keyword {
    If = 0,
    Define = 1,
    Lambda = 2,
    Quote = 3,
    True = 4,
    False = 5,
    SetBang = 6,
}

fn extract_param_ids(interp: &Interp, params: Value) -> Result<Vec<GcId>, SchemeError> {
    let ids = interp.heap.borrow().fold_list(
        interp, params, 
        Vec::new(), 
        |mut acc, param| {
            acc.push(interp.to_symbol(param)?);
            Ok(acc)
        }
    );
    ids
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
            6 => Some(Keyword::SetBang),
            _ => None,
        }
    }

    fn eval(interp: &Interp, env: &Rc<RefCell<Env>>, keyword: Keyword, args: &[Value]) -> Result<Value, SchemeError> {
        match keyword {
            Keyword::If => {
                if args.len() != 3 {
                    return Err(SchemeError::EvalError("if expects exactly 3 arguments".to_string()));
                }
                let condition = args[0].eval(interp, env)?;
                match condition {
                    Value::Boolean(true) => args[1].eval(interp, env),
                    Value::Boolean(false) => args[2].eval(interp, env),
                    _ => Err(SchemeError::TypeError("if condition must evaluate to a boolean".to_string())),
                }
            }
            Keyword::Define => {
                if args.len() != 2 {
                    return Err(SchemeError::EvalError("define! expects exactly 2 arguments".to_string()));
                }
                let var = &args[0];
                let value = args[1].eval(interp, env)?;
                if let Value::Object(var_id) = var {
                    env.borrow_mut().define(*var_id, value);
                    Ok(value)
                } else {
                    Err(SchemeError::TypeError("set! first argument must be a variable".to_string()))
                }
            }
            Keyword::Lambda => {
                match args {
                    [params_value, body @ ..] => {
                        let params = extract_param_ids(interp, *params_value)?;
                        let mut heap = interp.heap.borrow_mut();
                        let closure = heap.alloc_closure(Closure {
                            params: params.into_boxed_slice(),
                            body: body.to_vec().into_boxed_slice(),
                            env: Rc::clone(&interp.env),
                        });
                        Ok(closure) 
                    },
                    _ => Err(SchemeError::EvalError("lambda expects at least 2 arguments".to_string())),
                }
            }
            Keyword::Quote => {
                if args.len() != 1 {
                    return Err(SchemeError::EvalError("quote expects exactly 1 argument".to_string()));
                }
                Ok(args[0])
            }
            Keyword::SetBang => {
                if args.len() != 2 {
                    return Err(SchemeError::EvalError("set! expects exactly 2 arguments".to_string()));
                }
                let var = &args[0];
                let value = args[1].eval(interp, env)?;
                if let Value::Object(var_id) = var {
                    env.borrow_mut().set_bang(*var_id, value)?;
                    Ok(value)
                } else {
                    Err(SchemeError::TypeError("set! first argument must be a variable".to_string()))
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
        let set_bang_id = self.intern_symbol_to_gcid("set!");
        assert!(set_bang_id == Keyword::SetBang as usize, "Keyword 'set!' should have GcId 6");
    }

    pub fn get(&self, id: GcId) -> &HeapObject {
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

    pub fn alloc_pair(&mut self, car: Value, cdr: Value) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::Pair(car, cdr));
        Value::Object(id)
    }

    pub fn alloc_list(&mut self, items: Vec<Value>) -> Value {
        items.into_iter().rfold(Value::Nil, |acc, val| {
            self.alloc_pair(val, acc)
        })
    }

    pub fn fold_list<T, F>(&self, interp: &Interp, list: Value, init: T, mut func: F)  
        -> Result<T, SchemeError> 
        where 
        F: FnMut(T, Value) -> Result<T, SchemeError>
    {
        let mut p = list;
        let mut acc = init;
        while let Some((car, cdr)) = interp.is_pair(p) { 
            acc = func(acc, car)?;
            p = cdr;
        }
        Ok(acc)
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

    pub fn alloc_closure(&mut self, closure: Closure) -> Value {
        let id: GcId = self.objects.len();
        self.objects.push(HeapObject::Closure(Box::new(closure)));
        Value::Object(id)
    }

}
pub trait Apply {
    fn apply(&self, interp: &Interp, env: &Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, SchemeError>;
}

impl Apply for Value {
    fn apply(&self, interp: &Interp, _env: &Rc<RefCell<Env>>, args: Vec<Value>) -> Result<Value, SchemeError> {
        let obj = {
            let heap = interp.heap.borrow();
            match self {
                Value::Object(id) => heap.get(*id).clone(),
                _ => return Err(SchemeError::TypeError("Attempted to apply a non-object value".to_string())),
            }
        };
    
        match obj {
            HeapObject::Closure(closure) => {
                if closure.params.len() != args.len() {
                    return Err(SchemeError::EvalError("Incorrect number of arguments passed to closure".to_string()));
                }
                let new_env = Env::extend(closure.env.clone());
                for (param_id, arg_value) in closure.params.iter().zip(args.iter()) {
                    new_env.borrow_mut().define(*param_id, *arg_value);
                }
                let mut result = Value::Nil;
                for expr in &closure.body {
                    result = expr.eval(interp, &new_env)?;
                }
                Ok(result)
            },
            HeapObject::Primitive(pr) => pr(interp, &args),
            _ => Err(SchemeError::TypeError("Attempted to apply a non-primitive object".to_string())),
        }
    }
}



impl SchemeObject for GcId {

    fn eval(&self, interp: &Interp, env: &Rc<RefCell<Env>>) -> Result<Value, SchemeError> {
        let id = *self;
        let obj = {
            let heap = interp.heap.borrow();
            heap.get(id).clone()
        };
        
        match obj {
            HeapObject::Pair(car, cdr) => {
                if let Value::Object(func_id) = car 
                    && let Some(keyword) = Keyword::from_id(func_id) {
                    // Special form handling
                    let args = interp.heap.borrow().fold_list(
                        interp, 
                        cdr,
                        Vec::new(), 
                        |mut acc, arg| {
                            acc.push(arg);
                            Ok(acc)
                        });
                        Keyword::eval(interp, env, keyword, &args?)
                } else {
                    // Fallback if not a special form.
                    let args = interp.heap.borrow().fold_list(
                        interp, 
                        cdr,
                        Vec::new(), 
                        |mut acc, arg| {
                            let value = arg.eval(interp, env)?;
                            acc.push(value);
                            Ok(acc)
                        });
                    car.eval(interp, env)?.apply(interp, env, args?)
                }
            },
            HeapObject::List(elements) => {
                match elements.as_slice() {
                    [] => Ok(Value::Nil),
                    [func, rest @ ..] => {
                        if let Value::Object(func_id) = func 
                            && let Some(keyword) = Keyword::from_id(*func_id) {
                                // Special form handling
                                Keyword::eval(interp, env, keyword, rest)
                        } else {
                            // Fallback if not a pecial form.
                            let args = rest.iter()
                                .map(|arg| arg.eval(interp, env))
                                .collect::<Result<Vec<Value>, SchemeError>>()?;
                            func.eval(interp, env)?.apply(interp, env, args)
                        }
                    }    
                }
            },
            HeapObject::Symbol(name) => {
                match env.borrow().lookup(id) {
                    Some(value) => return Ok(value),
                    None => {
                        return Err(SchemeError::UnboundVariable(format!("Unbound symbol: {}", name)))
                    },
                }
            },
            HeapObject::FreeSlot(_) => Err(SchemeError::ImplementationError(format!(
                "Request to evaluate FreeSlot at {}", id
            ))),
            _ => Ok(Value::Object(id))
        }
    }

    fn is_false(&self) -> bool {
        return *self == Keyword::False as usize;
    }
    
    fn write_to(&self, interp: &Interp, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = *self;
        let heap = interp.heap.borrow();
        let obj = heap.get(id);
        match obj {
            HeapObject::Pair(car, cdr) => {
                let mut p = cdr.clone();
                write!(f, "(")?;
                car.write_to(interp, f)?;
                loop {
                    if let Some((cadr, cddr)) = interp.is_pair(p) { 
                        write!(f, " ")?;
                        cadr.write_to(interp, f)?;
                        p = cddr;
                    } else if interp.is_nil(p) {
                        break;
                    } else {
                        write!(f, " . ")?;
                        p.write_to(interp, f)?;
                    }
                }
                write!(f, ")")
            },
            HeapObject::List(elements) => {
                write!(f, "(")?;
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?; // Add a space before every element EXCEPT the first
                    }
                    e.write_to(interp, f)?;
                }
                write!(f, ")")
            },
            HeapObject::Symbol(s) => write!(f, "{}", s),
            HeapObject::String(s) => write!(f, "\"{}\"", s),
            HeapObject::Primitive(pr) => write!(f, "<primitive {:p}>", pr),
            HeapObject::Closure(_) => write!(f, "<closure {}>", id),
            HeapObject::FreeSlot(_) => write!(f, "*** FREE SLOT ***")
        }
    }
}