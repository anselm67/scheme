use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    check_arity, env::Env, interp::Interp, markset::MarkSet, types::{GcId, SchemeError, SchemeObject, Value}
};

pub type PrimitiveFn = fn(&Interp, env: &Rc<RefCell<Env>>, &[Value]) -> Result<Value, SchemeError>;


#[derive(Clone)]
pub struct Closure {
    params: Box<[GcId]>,
    body: Box<[Value]>,
    env: Rc<RefCell<Env>>,
}

#[derive(Clone)]
pub struct Vector {
    pub data: RefCell<Vec<Value>>,
}

#[derive(Clone)]
pub enum HeapObject {
    FreeSlot(GcId),
    Pair(Value, Value),
    Vector(Vector),
    Symbol(String),
    String(String),
    Primitive(PrimitiveFn),
    Closure(Box<Closure>),
    NaryClosure(Box<Closure>),
    // Other heap-allocated object types can be added here
}

impl HeapObject {

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::FreeSlot(_) => "FreeSlot",
            Self::Pair(..) => "Pair",
            Self::Vector(_) => "Vector",
            Self::Symbol(_) => "Symbol",
            Self::String(_) => "String",
            Self::Primitive(_) => "Primitive",
            Self::Closure(_) => "Closure",
            Self::NaryClosure(_) => "n-Closure",
        }
    }

    pub fn is_equal(&self, interp: &Interp, other: &HeapObject) -> bool {
        match (self, other) {
            (HeapObject::FreeSlot(_), HeapObject::FreeSlot(_)) => false,
            (HeapObject::Pair(acar, acdr), HeapObject::Pair(bcar, bcdr)) => {
                acar.is_equal(interp, bcar) && acdr.is_equal(interp, bcdr)
            },
            (HeapObject::Vector(v1), HeapObject::Vector(v2)) => {
                let d1 = v1.data.borrow();
                let d2 = v2.data.borrow();
                d1.len() == d2.len() && d1.iter().zip(d2.iter())
                    .all(|(a, b)| a.is_equal(interp, b))
            },
            (HeapObject::Symbol(a), HeapObject::Symbol(b)) => {
                a == b
            },
            (HeapObject::String(a), HeapObject::String(b)) => a == b,
            (HeapObject::Primitive(p1), HeapObject::Primitive(p2)) => {
                std::ptr::eq(p1,p2)
            },
            (HeapObject::Closure(c1), HeapObject::Closure(c2)) => {
                std::ptr::eq(c1, c2)
            }
            (HeapObject::NaryClosure(p1), HeapObject::NaryClosure(p2)) => {
                std::ptr::eq(p1, p2)
            },
            _ => false,
        }
    }

}

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Keyword {
    If = 0,
    DefineBang = 1,
    Lambda = 2,
    Quote = 3,
    True = 4,
    False = 5,
    SetBang = 6,
    QuasiQuote = 7,
    DefineSyntax = 8,
}

fn extract_param_ids(interp: &Interp, params: Value) -> Result<(Vec<GcId>, bool), SchemeError> {
    let mut ids = Vec::new();
    let mut p = params;
    let mut is_nary = false;

    if let Some(id) = interp.is_symbol(params) {
        ids.push(id);
        is_nary = true;
    } else {
        while let Some((car, cdr)) = interp.is_pair(p) { 
            ids.push(interp.to_symbol(car)?);
            if interp.is_nil(cdr) {
                break;
            } else if interp.is_pair(cdr).is_some() {
                p = cdr;
            } else {
                is_nary = true;
                ids.push(interp.to_symbol(cdr)?);
                break;
            }
        }
    }
    Ok((ids, is_nary))
}


impl Keyword {

    fn from_id(id: GcId) -> Option<Keyword> {
        match id {
            0 => Some(Keyword::If),
            1 => Some(Keyword::DefineBang),
            2 => Some(Keyword::Lambda),
            3 => Some(Keyword::Quote),
            4 => Some(Keyword::True),
            5 => Some(Keyword::False),
            6 => Some(Keyword::SetBang),
            7 => Some(Keyword::QuasiQuote),
            8 => Some(Keyword::DefineSyntax),
            _ => None,
        }
    }

    fn eval(interp: &Interp, env: &Rc<RefCell<Env>>, keyword: Keyword, args: &[Value]) -> Result<Value, SchemeError> {
        match keyword {
            Keyword::If => {
                check_arity!(args, 3);
                let condition = args[0].eval(interp, env)?;
                match condition {
                    Value::Boolean(false) => args[2].eval(interp, env),
                    _ => args[1].eval(interp, env),
                }
            },
            Keyword::DefineBang => {
                check_arity!(args, 2);
                let symbol = interp.to_object(args[0])?;
                let value = args[1].eval(interp, env)?;
                env.borrow_mut().define(symbol, value);
                Ok(Value::Nil)
            },
            Keyword::DefineSyntax => {
                check_arity!(args, 2);
                let symbol = interp.to_symbol(args[0])?;
                let value = args[1].eval(interp, env)?;
                env.borrow_mut().define_syntax(symbol, value);
                Ok(Value::Nil)
            },
            Keyword::Lambda => {
                match args {
                    [params_value, body @ ..] => {
                        let (params, is_nary) = extract_param_ids(interp, *params_value)?;
                        let mut heap = interp.heap.borrow_mut();
                        if is_nary {
                            Ok(heap.alloc_nary_closure(Closure {
                                params: params.into_boxed_slice(),
                                body: body.to_vec().into_boxed_slice(),
                                env: Rc::clone(&interp.env),
                            }))
                        } else {
                            Ok(heap.alloc_closure(Closure {
                                params: params.into_boxed_slice(),
                                body: body.to_vec().into_boxed_slice(),
                                env: Rc::clone(&interp.env),
                            }))
                        }
                    },
                    _ => Err(SchemeError::EvalError(format!(
                        "lambda expects at least 2 arguments, got {}", args.len()
                    ))),
                }
            }
            Keyword::Quote => {
                check_arity!(args, 1);
                Ok(args[0])
            }
            Keyword::QuasiQuote => {
                check_arity!(args, 1);
                let expr = interp.expand_quasiquote(args[0])?;
                interp.eval(env, expr)
            },
            Keyword::SetBang => {
                check_arity!(args, 2);
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
    size: usize,
    free_slot: usize
}

pub struct HeapStats {
    pub total_slots: usize,
    pub live_slots: usize,
    pub next_slot: usize,
    pub free_slots: usize,
    pub symbol_count: usize,
}

impl Heap {
    
    pub fn new(size: usize) -> Self {
        let mut heap = Self {
            objects: vec![HeapObject::FreeSlot(0); size],
            symbols: HashMap::new(),
            size: size,
            free_slot: 0,
        };
        // Chain all slots into free slots.
        // FreeSlot(i) if i >= size means we've reached the end.
        for i in 0..size {
            heap.objects[i] = HeapObject::FreeSlot(i+1);
        }
        // Pre-intern keywords
        heap.intern_special_keywwords();
        heap
    }

    fn next_id(&mut self) -> GcId {
        if self.free_slot < self.size {
            let available_id = self.free_slot;
            if let HeapObject::FreeSlot(free_slot) = self.objects[self.free_slot] {
                self.free_slot = free_slot;
            } else {
                panic!("Free slot {} is occupied by a {} !", 
                    self.free_slot, self.objects[self.free_slot].type_name())
            }
            return available_id;
        }
        // TODO Run the (gc) to get some space.
        panic!("No more memory !");
    }

    pub fn stats(&self) -> HeapStats {
        let free_count = self.objects.iter()
            .filter(|slot| matches!(slot, HeapObject::FreeSlot(_)))
            .count();
        HeapStats {
            total_slots: self.objects.len(),
            live_slots: self.size - free_count,
            next_slot: self.free_slot,
            free_slots: free_count,
            symbol_count: self.symbols.len()
        }
    }

    fn intern_special_keywwords(&mut self) {
        // TODO Cleanup indent & line breaks.
        let if_id =self.intern_symbol_to_gcid("if");
        assert!(if_id == Keyword::If as usize, "Keyword 'if' should have GcId 0");
        let define_id = self.intern_symbol_to_gcid("define!");
        assert!(define_id == Keyword::DefineBang as usize, "Keyword 'define!' should have GcId 1");
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
        let quasiquote_id = self.intern_symbol_to_gcid("quasiquote");
        assert!(quasiquote_id == Keyword::QuasiQuote as usize, "Keyword 'quasiquote' should have GcId 7");
        let define_syntax_id = self.intern_symbol_to_gcid("define-syntax");
        assert!(define_syntax_id == Keyword::DefineSyntax as usize, "Keyword 'define-syntax' should have GcId 8");
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn get(&self, id: GcId) -> &HeapObject {
        &self.objects[id]
    }

    pub fn get_mut(&mut self, id: GcId) -> &mut HeapObject {
        &mut self.objects[id]
    }

    fn intern_symbol_to_gcid(&mut self, name: &str) -> GcId {
        if let Some(&id) = self.symbols.get(name) {
            return id;
        } else {
            let id: GcId = self.next_id();
            self.objects[id] = HeapObject::Symbol(name.to_string());
            self.symbols.insert(name.to_string(), id);
            id
        }
    }
    
    pub fn intern_symbol(&mut self, name: &str) -> Value {
        Value::Object(self.intern_symbol_to_gcid(name))
    }

    pub fn alloc_pair(&mut self, car: Value, cdr: Value) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::Pair(car, cdr);
        Value::Object(id)
    }

    pub fn last(&self, car: Value) -> Result<Value, SchemeError> {
        let mut tail = car;
        while let Value::Object(id) = tail {
            match self.get(id) {
                HeapObject::Pair(_, cdr) => {
                    if matches!(cdr, Value::Nil) {
                        return Ok(tail);
                    } else {
                        tail = *cdr;
                    }
                },
                _ => break
            }
        } 
        return Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {}.", car.type_name()
            )));
    }

    pub fn setcdr(&mut self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        match self.get_mut(id) {
            HeapObject::Pair(_, cdr) => {
                *cdr = value;
                Ok(value)
            },
            obj => Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {} instead.", obj.type_name()
            )))
        }
    }

    pub fn alloc_list(&mut self, items: &[Value]) -> Value {
        items.into_iter().rfold(Value::Nil, |acc, val| {
            self.alloc_pair(*val, acc)
        })
    }

    pub fn alloc_list_with_cdr(&mut self, items: &[Value], cdr: Value) -> Value {
        items.into_iter().rfold(cdr, |acc, val| {
            self.alloc_pair(*val, acc)
        })
    }

    pub fn alloc_string(&mut self, s: impl Into<String>) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::String(s.into());
        Value::Object(id)
    }

    pub fn alloc_primitive(&mut self, func: PrimitiveFn) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::Primitive(func);
        Value::Object(id)
    }

    pub fn alloc_closure(&mut self, closure: Closure) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::Closure(Box::new(closure));
        Value::Object(id)
    }

    pub fn alloc_nary_closure(&mut self, closure: Closure) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::NaryClosure(Box::new(closure));
        Value::Object(id)
    }

    pub fn alloc_vector(&mut self, items: &[Value]) -> Value {
        let id: GcId = self.next_id();
        self.objects[id] = HeapObject::Vector(Vector { data: RefCell::new(items.to_vec()) });
        Value::Object(id)
    }
    
    pub fn mark(&self, interp: &Interp, marks: &mut MarkSet) {
        for id in self.symbols.values() {
            id.mark(interp, marks);
        }
    }

    fn make_free_slot(&mut self, id: GcId) {
        self.objects[id] = HeapObject::FreeSlot(self.free_slot);
        self.free_slot = id;
    }

    pub fn collect(&mut self, marks: &MarkSet) -> usize {
        let mut count = 0;
        // Display the objects we'd like to kill:
        for id in 0..marks.len() {
            if ! marks.is_marked(id) && ! matches!(self.objects[id], HeapObject::FreeSlot(_)) {
                self.make_free_slot(id);
                count += 1;
            }
        }
        count
    }
}
pub trait Apply {
    fn apply(&self, interp: &Interp, env: &Rc<RefCell<Env>>, args: Vec<Value>) 
        -> Result<Value, SchemeError>;
}

impl Apply for Value {
    fn apply(&self, interp: &Interp, env: &Rc<RefCell<Env>>, args: Vec<Value>) 
        -> Result<Value, SchemeError> 
    {
        let obj = {
            let heap = interp.heap.borrow();
            match self {
                Value::Object(id) => heap.get(*id).clone(),
                _ => return Err(SchemeError::TypeError(format!(
                    "Attempted to apply a non-object value with type {}", self.type_name()
                ))),
            }
        };
    
        match obj {
            HeapObject::Pair(car, _) => {
                let func = car.eval(interp, env)?;
                func.apply(interp, env, args)
            },
            HeapObject::Closure(closure) => {
                check_arity!(args, closure.params.len());
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
            HeapObject::NaryClosure(closure) => {
                let new_env = Env::extend(closure.env.clone());
                let mut index = 0;
                if args.len() < closure.params.len() - 1 {
                    return Err(SchemeError::ArgCountError(format!(
                        "Expected at least {} args, but got {}.", closure.params.len() - 1, args.len()
                    )))
                }
                while index < closure.params.len() - 1 {
                    new_env.borrow_mut().define(closure.params[index], args[index]);
                    index += 1;
                }
                let rest = interp.heap.borrow_mut().alloc_list(&args[index..]);
                new_env.borrow_mut().define(closure.params[index], rest);
                let mut result = Value::Nil;
                for expr in &closure.body {
                    result = expr.eval(interp, &new_env)?;
                }
                Ok(result)
            },
            HeapObject::Primitive(pr) => pr(interp, env, &args),
            HeapObject::FreeSlot(_) => {
                panic!("Attempt to apply a FreeSlot!");
            }
            any => Err(SchemeError::TypeError(format!(
                "Attempted to apply a non-primitive object with type {}", any.type_name()
            ))),
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
                    && let Some(keyword) = Keyword::from_id(func_id) 
                {
                    // Special form handling - no args eval.
                    let args = interp.fold_list(
                        cdr,
                        Vec::new(), 
                        |mut acc, arg| {
                            acc.push(arg);
                            Ok(acc)
                        });
                    Keyword::eval(interp, env, keyword, &args?)
                } else {
                    // Regular function call with arg eval.
                    let args = interp.fold_list(
                        cdr,
                        Vec::new(), 
                        |mut acc, arg| {
                            let value = arg.eval(interp, env)?;
                            acc.push(value);
                            Ok(acc)
                        });
                    let func = car.eval(interp, env)?;
                    func.apply(interp, env, args?)
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
            HeapObject::FreeSlot(_) => panic!("Request to evaluate FreeSlot at {}", id),
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
                        break;
                    }
                }
                write!(f, ")")
            },
            HeapObject::Vector(v) => {
                let items = v.data.borrow();
                write!(f, "#(")?;
                for (i, e) in items.iter().enumerate() {
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
            HeapObject::NaryClosure(_) => write!(f, "<n-closure {}>", id),
            HeapObject::FreeSlot(id) => panic!("Attempt to render free slot {}", id),
        }
    }

    fn mark(&self, interp: &Interp, marks: &mut MarkSet) {
        // If we've already been marked, no need to walk through again.
        let id = *self;
        if ! marks.mark(id) {
            return;
        }
        let obj = {
            let heap = interp.heap.borrow();
            heap.get(id).clone()
        };
        match obj {
            HeapObject::Pair(car, cdr) => {
                car.mark(interp, marks);
                cdr.mark(interp, marks);
            },
            HeapObject::Vector(v) => {
                let data = v.data.borrow();
                for item in data.iter() {
                    item.mark(interp, marks);
                }
            },
            HeapObject::Symbol(_) => {},
            HeapObject::String(_) => {},
            HeapObject::Primitive(_) => {},
            HeapObject::Closure(closure) | HeapObject::NaryClosure(closure) => {
                for id in closure.params {
                    id.mark(interp, marks);
                }
                for expr in closure.body {
                    expr.mark(interp, marks);
                }
                closure.env.borrow().mark(interp, marks);
            },
            _ => {
                panic!("Request to mark a {}.", obj.type_name());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_pair() {
        let mut heap = Heap::new(1024);
        let pair = heap.alloc_list(&[Value::Boolean(true)]);
        assert!(matches!(pair, Value::Object(_)));
        if let Value::Object(id) = pair {
            let obj = heap.get(id);
            assert!(matches!(*obj, HeapObject::Pair(..)));
            if let HeapObject::Pair(car, cdr) = obj {
                assert_eq!(*car, Value::Boolean(true));
                assert_eq!(*cdr, Value::Nil);
            }
        }
    }

    #[test]
     fn test_alloc_pair_with_cdr() {
        let mut heap = Heap::new(1024);
        let pair = heap.alloc_list_with_cdr(
            &[Value::Boolean(true)], Value::Boolean(false)
        );
        assert!(matches!(pair, Value::Object(_)));
        if let Value::Object(id) = pair {
            let obj = heap.get(id);
            assert!(matches!(*obj, HeapObject::Pair(..)));
            if let HeapObject::Pair(car, cdr) = obj {
                assert_eq!(*car, Value::Boolean(true));
                assert_eq!(*cdr, Value::Boolean(false));
            }
        }
    }
}