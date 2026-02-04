use std::cell::RefCell;
use std::collections::HashMap;
use std::process;
use std::rc::Rc;

use crate::heap::HeapObject;
use crate::{all_of_type, extract_args, heap};
use crate::types::{GcId, Number, SchemeError, SchemeObject, Value};

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
        // Initialize math primitive functions
        self.define_primitive("number?", primitive_number_p);
        self.define_primitive("integer?", primitive_integer_p);
        self.define_primitive("float?", primitive_float_p);
        self.define_primitive("+", primitive_add);
        self.define_primitive("-", primitive_sub);
        self.define_primitive("*", primitive_mul);
        self.define_primitive("/", primitive_div);
        self.define_primitive("%", primitive_rem);
        self.define_primitive("=", primitive_number_eq);
        self.define_primitive("<", primitive_number_lt);
        self.define_primitive(">", primitive_number_gt);
        self.define_primitive("<=", primitive_number_lte);
        self.define_primitive(">=", primitive_number_gte);
        self.define_primitive("max", primitive_number_max);
        self.define_primitive("min", primitive_number_min);

        // Initialize list functions.
        self.define_primitive("list", primitive_list);
        self.define_primitive("list?", primitive_list_p);
        self.define_primitive("null?", primitive_null_p);
        // self.define_primitive("cons", primitive_list_cons);
        self.define_primitive("car", primitive_list_car);
        self.define_primitive("cdr", primitive_list_cdr);

        // Initialize system primitive functions.
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

    pub fn is_nil(&self, value: Value) -> bool {
        matches!(value, Value::Nil)
    }

    pub fn is_list(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            matches!(self.heap.borrow().get(id), HeapObject::Pair(..))
        } else if matches!(value, Value::Nil) {
            true
        } else {
            false
        }
    }

    pub fn is_null(&self, value: Value) -> bool {
        matches!(value, Value::Nil)
    }

    pub fn is_integer(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(n @ Number::Int(_)) => Some(n),
            _ => None
        }
    }

    pub fn is_float(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(f @ Number::Float(_)) => Some(f),
            _ => None
        }
    }

    pub fn is_object(&self, value: Value) -> Option<GcId> {
        match value {
            Value::Object(id) => Some(id),
            _ => None,
        }
    }

    pub fn to_object(&self, value: Value) -> Result<GcId, SchemeError> {
        match value {
            Value::Object(id) => Ok(id),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an Object, got a {}", value.type_name()
            ))),
        }
    }

    pub fn is_pair(&self, value: Value) -> Option<(Value, Value)> {
        if let Some(id) = self.is_object(value) {
            match self.heap.borrow().get(id) {
                HeapObject::Pair(car, cdr) => Some((*car, *cdr)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn to_pair(&self, value: Value) -> Result<(Value, Value), SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::Pair(car, cdr) => Ok((*car, *cdr)),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {}.", value.type_name()))),
        }
    }

    pub fn to_symbol(&self, value: Value) -> Result<GcId, SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::Symbol(_) => Ok(id),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Symbol, but got a {}.", value.type_name()
            )))
        }
    }

}

fn primitive_add(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let sum = nums.into_iter()
        .fold(Number::Int(0), |acc, n| acc  + n);
    Ok(Value::Number(sum))
}

fn primitive_sub(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "- expects at least one arg.".to_string()
        ))
    }

    let mut iter = nums.into_iter();
    let init = iter.next().unwrap();
    let sub = if let None = iter.clone().next() {
        - init
    } else {
        iter.fold(init, |acc, n| acc - n)
    };
    Ok(Value::Number(sub))
}

fn primitive_div(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "- expects at least one arg.".to_string()
        ))
    }

    let mut iter = nums.into_iter();
    let init = iter.next().unwrap();
    let div = if let None = iter.clone().next() {
        Number::Float(1.0) / init
    } else {
        iter.fold(init, |acc, n| acc / n)
    };
    Ok(Value::Number(div))
}


fn primitive_mul(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let mul = nums.into_iter()
        .fold(Number::Int(1), |acc, n| acc * n);
    Ok(Value::Number(mul))
}

fn primitive_rem(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(Value::Number(*a % *b))
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

fn primitive_number_lt(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    return Ok(Value::Boolean(a < b))
}

fn primitive_number_lte(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    return Ok(Value::Boolean(a <= b))
}

fn primitive_number_gt(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    return Ok(Value::Boolean(a > b))
}

fn primitive_number_gte(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    return Ok(Value::Boolean(a >= b))
}

fn primitive_number_p(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.is_empty() {
        return Err(SchemeError::ArgCountError(
            "numberp? expects exactly one arg.".to_string()));
    }
    match args[0] {
        Value::Number(_) => Ok(Value::Boolean(true)),
        _ => Ok(Value::Boolean(false))
    }
}

fn primitive_integer_p(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.is_empty() {
        return Err(SchemeError::ArgCountError(
            "integer? expects exactly one arg.".to_string()));
    }
    Ok(Value::Boolean(interp.is_integer(args[0]).is_some()))
}

fn primitive_float_p(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.is_empty() {
        return Err(SchemeError::ArgCountError(
            "float? expects exactly one arg.".to_string()));
    }
    Ok(Value::Boolean(interp.is_float(args[0]).is_some()))
}

fn primitive_number_max(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "max expects at least one arg.".to_string()));
    }
    let init = nums[0];
    let ret = nums.into_iter()
        .fold(init, |a, b| if a > b { a } else { b });
    Ok(Value::Number(ret))
}

fn primitive_number_min(_interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "min expects at least one arg.".to_string()));
    }
    let init = nums[0];
    let ret = nums.into_iter()
        .fold(init, |a, b| if a < b { a } else { b });
    Ok(Value::Number(ret))
}

fn primitive_list(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.is_empty() {
        Ok(Value::Nil)
    } else {
        Ok(interp.heap.borrow_mut().alloc_list(args[1..].to_vec()))
    }
}

fn primitive_list_p(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.is_empty()  {
        return Err(SchemeError::ArgCountError(format!(
            "list? takes exactly one arg, but got {}.", args.len()
        )));
    }
    Ok(Value::Boolean(interp.is_list(args[0])))
}

fn primitive_null_p(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    if args.len() != 1 {
        Err(SchemeError::ArgCountError(format!(
            "list? expects one arg, but got {}", args.len()
        )))
    } else {
        Ok(Value::Boolean(interp.is_null(args[0])))
    }
}

// fn primitive_cons(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {

// }

fn primitive_list_car(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 1, id: Object);
    let list = interp.heap.borrow().get(*id).clone();
    match list {
        HeapObject::List(v) => {
            if v.is_empty() {
                return Err(SchemeError::EvalError(
                    "Can't take the car of an empty list.".to_string()
                ));
            } else {
                Ok(v[0])
            }
        }
        _ => return Err(SchemeError::TypeError(format!(
            "Invalid type {} for cdr, expecting a List", list.type_name()
        )))
    }
}

fn primitive_list_cdr(interp: &Interp, args: &[Value]) -> Result<Value, SchemeError> {
    extract_args!(args, 1, id: Object);
    let list = interp.heap.borrow().get(*id).clone();
    match list {
        HeapObject::List(v) => {
            if v.is_empty() {
                return Err(SchemeError::EvalError(
                    "Can't take the cdr of an empty list.".to_string()
                ));
            } else {
                let mut heap = interp.heap.borrow_mut();
                Ok(heap.alloc_list(v[1..].to_vec()))
            }
        }
        _ => return Err(SchemeError::TypeError(format!(
            "Invalid type {} for cdr, expecting a List", list.type_name()
        )))
    }
}
