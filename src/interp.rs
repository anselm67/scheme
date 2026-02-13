use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process;
use std::rc::Rc;

use crate::env::Env;
use crate::heap::{Apply, HeapObject, Keyword, Vector};
use crate::markset::MarkSet;
use crate::parser::Parser;
use crate::{all_of_type, check_arity, check_min_arity, extract_args, heap};
use crate::types::{DisplayWrapper, EvalResult, GcId, Number, SchemeError, SchemeObject, Value};

pub struct Interp {
    pub heap: RefCell<heap::Heap>,
    pub env: Rc<RefCell<crate::env::Env>>,

    // Some symbols we want to keeep track of:
    append: Value,
    list: Value,
    quasiquote: Value,
    unquote: Value,
    unquote_splicing: Value,
}

impl Interp {
    pub fn new() -> Self {
        let global_env = crate::env::Env {
            macros: HashMap::new(),
            bindings: HashMap::new(),
            parent: None,
        };
        let env_handle = Rc::new(RefCell::new(global_env));
        let heap_handle = RefCell::new(heap::Heap::new(4096));
        let (append, list, quasiquote, unquote, unquote_splicing) = {
            let mut heap = heap_handle.borrow_mut();
            (
                heap.intern_symbol("append"),
                heap.intern_symbol("list"),
                heap.intern_symbol("quasiquote"),
                heap.intern_symbol("unquote"),
                heap.intern_symbol("unquote-splicing"),
            )
        };
        let interp = Self {
            heap: heap_handle,
            env: env_handle,

            list: list,
            append: append,
            quasiquote: quasiquote,
            unquote: unquote,
            unquote_splicing: unquote_splicing
        };  
        interp.init();
        interp
    }

    pub fn define(&self, name: &str, value: Value) -> Value {
        let symbol = self.heap.borrow_mut().intern_symbol(name);
        if let Value::Object(id) = symbol {
            self.env.borrow_mut().define(id, value);
        }
        symbol
    }

    pub fn define_primitive(&self, name: &str, func: heap::PrimitiveFn) {
        let prim = self.heap.borrow_mut().alloc_primitive(func);
        self.define(name, prim);
    }

    fn init(&self) {
        self.define_primitive("eval", primitive_eval);
        self.define_primitive("apply", primitive_apply);
        self.define_primitive("expand", primitive_expand);
        self.define_primitive("eq?", primitive_eq);
        self.define_primitive("equal?", primitive_equal);
        self.define_primitive("error", primitive_error);

        self.define("#t", Value::Boolean(true));
        self.define("#f", Value::Boolean(false));

        // Initialize symbol functions.
        self.define_primitive("symbol?", primitive_symbol_p);

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


        // Initialize character functions.
        self.define_primitive("char?", primitive_char_p);
        self.define_primitive("char-alphabetic?", primitive_char_alphabetic_p);
        self.define_primitive("char-numeric?", primitive_char_numeric_p);
        self.define_primitive("char-whitespace?", primitive_char_whitespace_p);
        self.define_primitive("char-upper-case?", primitive_char_upper_case_p);
        self.define_primitive("char-lower-case?", primitive_char_lower_case_p);
        self.define_primitive("char->integer", primitive_char_to_integer);
        self.define_primitive("integer->char", primitive_integer_to_char);
        self.define_primitive("char-upcase", primitive_char_upcase);
        self.define_primitive("char-downcase", primitive_char_downcase);
        self.define_primitive("char=?", primitive_char_eq);
        self.define_primitive("char<?", primitive_char_lt);
        self.define_primitive("char<=?", primitive_char_lte);
        self.define_primitive("char>?", primitive_char_gt);
        self.define_primitive("char>=?", primitive_char_gte);
        self.define_primitive("char-ci=?", primitive_char_ci_eq);
        self.define_primitive("char-ci<?", primitive_char_ci_lt);
        self.define_primitive("char-ci<=?", primitive_char_ci_lte);
        self.define_primitive("char-ci>?", primitive_char_ci_gt);
        self.define_primitive("char-ci>=?", primitive_char_ci_gte);


        // Initialize list functions.
        self.define_primitive("list", primitive_list);
        self.define_primitive("append", primitive_append);
        self.define_primitive("length", primitive_length);
        self.define_primitive("list?", primitive_list_p);
        self.define_primitive("null?", primitive_null_p);
        self.define_primitive("cons", primitive_list_cons);
        self.define_primitive("car", primitive_list_car);
        self.define_primitive("cdr", primitive_list_cdr);

        // Initializes vector functions.
        self.define_primitive("vector?", primitive_vector_p);
        self.define_primitive("make-vector", primitive_make_vector);
        self.define_primitive("vector", primitive_vector);
        self.define_primitive("vector-length", primitive_vector_length);
        self.define_primitive("vector-ref", primitive_vector_ref);
        self.define_primitive("vector-set!", primitive_vector_set);
        self.define_primitive("vector->list", primitive_vector_to_list);
        self.define_primitive("list->vector", primitive_list_to_vector);
        self.define_primitive("vector-fill!", primitive_vector_fill);

        // Initializes string functions.
        self.define_primitive("string?", primitive_string_p);
        self.define_primitive("make-string", primitive_make_string);
        self.define_primitive("string", primitive_string);
        self.define_primitive("string->list", primitive_string_to_list);
        self.define_primitive("list->string", primitive_list_to_string);
        self.define_primitive("string-length", primitive_string_length);
        self.define_primitive("string-ref", primitive_string_ref);
        self.define_primitive("string-set!", primitive_string_set);
        self.define_primitive("string=?", primitive_string_eq);
        self.define_primitive("string<?", primitive_string_lt);
        self.define_primitive("string<=?", primitive_string_lte);
        self.define_primitive("string>?", primitive_string_gt);
        self.define_primitive("string>=?", primitive_string_gte);
        self.define_primitive("string-ci=?", primitive_string_ci_eq);
        self.define_primitive("string-ci<?", primitive_string_ci_lt);
        self.define_primitive("string-ci<=?", primitive_string_ci_lte);
        self.define_primitive("string-ci>?", primitive_string_ci_gt);
        self.define_primitive("string-ci>=?", primitive_string_ci_gte);
        self.define_primitive("string-append", primitive_string_append);
        self.define_primitive("substring", primitive_substring);
        self.define_primitive("string-copy", primitive_string_copy);
        self.define_primitive("string-fill!", primitive_string_fill);

        // IO primitive functions.
        self.define_primitive("open-input-file", primitive_open_input_file);
        self.define_primitive("close-input-port", primitive_close_input_port);
        self.define_primitive("read-char", primitive_read_char);
        self.define_primitive("eof-object?", primitive_eof_object);

        // Initialize system primitive functions.
        self.define_primitive("gc", primitive_gc);
        self.define_primitive("heap-stats", primitive_heap_stats);
        self.define_primitive("debug", primitive_debug);
        self.define_primitive("load", primitive_load);
        self.define_primitive("quit", primitive_quit);
        self.define_primitive("exit", primitive_quit);
    }

    pub fn fold_list<T, F>(&self, list: Value, init: T, mut func: F)  
        -> Result<T, SchemeError> 
        where 
        F: FnMut(T, Value) -> Result<T, SchemeError>
    {
        let mut p = list;
        let mut acc = init;
        while let Some((car, cdr)) = self.is_pair(p) { 
            acc = func(acc, car)?;
            p = cdr;
        }
        Ok(acc)
    }


    pub fn lookup(&self, name: &str) -> Value {
        self.heap.borrow_mut().intern_symbol(name)
    }

    pub fn eval(&self, env: Rc<RefCell<Env>>, expr: Value)  
        -> Result<Value, SchemeError> 
    {
        let mut current_expr = expr;
        let mut current_env = env;
        loop {
            match current_expr.eval(self, current_env)?  {
                EvalResult::Done(value) => return Ok(value),
                EvalResult::Continuation(next_env, next_expr) => {
                    current_expr = next_expr;
                    current_env = next_env;
                }
            }
        }
    }

    // TODO Remove this function all together.
    pub fn eval_full(&self, env: Rc<RefCell<Env>>, obj: Value)  
        -> Result<Value, SchemeError> 
    {
        self.eval(env, obj)
    }

    pub fn display(&self, obj: Value) -> String {
        let wrapper = DisplayWrapper{ obj: &obj, interp: self };
        wrapper.to_string()
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

    pub fn to_integer(&self, value: Value) -> Result<i64, SchemeError> {
        match value {
            Value::Number(Number::Int(i)) => Ok(i),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an Number::Int, but got a {}.", value.type_name()
            )))
        }
    }

    pub fn is_float(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(f @ Number::Float(_)) => Some(f),
            _ => None
        }
    }

    pub fn is_number(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    pub fn is_char(&self, value: Value) -> Option<u8> {
        match value {
            Value::Char(ch) => Some(ch),
            _ => None,
        }
    }

    pub fn to_char(&self, value: Value) -> Result<char, SchemeError> {
        match value {
            Value::Char(ch) => Ok(ch as char),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Char got a {}", value.type_name()
            ))),
        }
    }

    pub fn is_string(&self, value: Value) -> Option<Ref<'_, String>> {
        if let Some(id) = self.is_object(value) {
            Ref::filter_map(self.heap.borrow(), |h| {
                if let HeapObject::String(string) = h.get(id) {
                    Some(string)
                } else {
                    None
                }
            }).ok()
        } else {
            None
        }
    }

    pub fn to_string(&self, value: Value) -> Result<Ref<'_,String>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        Ref::filter_map(heap, |h| {
            if let HeapObject::String(string) = h.get(id) {
                Some(string)
            } else {
                None
            }
        }).map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected a String, but got a {}", value.type_name()
            ))
        })
    }

    pub fn to_string_mut(&self, value: Value) -> Result<RefMut<'_,String>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow_mut();
        RefMut::filter_map(heap, |h| {
            if let HeapObject::String(string) = h.get_mut(id) {
                Some(string)
            } else {
                None
            }
        }).map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected a String, but got a {}", value.type_name()
            ))
        })
    }

    pub fn is_vector(&self, value: Value) -> Option<Ref<'_, Vector>> {
        if let Some(id) = self.is_object(value) {
            Ref::filter_map(self.heap.borrow(), |h| {
                if let HeapObject::Vector(vector) = h.get(id) {
                    Some(vector)
                } else {
                    None
                }
            }).ok()
        } else {
            None
        }
    }

    pub fn to_vector(&self, value: Value) -> Result<Ref<'_,Vector>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        Ref::filter_map(heap, |h| {
            if let HeapObject::Vector(vector) = h.get(id) {
                Some(vector)
            } else {
                None
            }
        }).map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected a Vector, but got a {}", value.type_name()
            ))
        })
    }

    pub fn to_input_port(&self, value: Value) 
        -> Result<Ref<'_,Rc<RefCell<Option<Box<dyn BufRead>>>>>, SchemeError> 
    {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        Ref::filter_map(heap, |h| {
            if let HeapObject::InputPort(input) = h.get(id) {
                Some(input)
            } else {
                None
            }
        }).map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected an InputPort, but got a {}", value.type_name()
            ))
        })        
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

    pub fn to_car(&self, value: Value)  -> Result<Value, SchemeError> {
        let (car, _) = self.to_pair(value)?;
        Ok(car)
    }

    pub fn is_symbol(&self, value: Value) -> Option<GcId> {
        if let Some(id) = self.is_object(value) {
            if matches!(self.heap.borrow().get(id), HeapObject::Symbol(_)) {
                return Some(id);
            }
        } 
        None
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

    pub fn quote(&self, obj: Value) -> Result<Value, SchemeError> {
        let value = &[
            Value::Object(Keyword::Quote as usize), 
            obj,
        ];
        Ok(self.heap.borrow_mut().alloc_list(value))
    }

    pub fn quasiquote(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.quasiquote, obj]))
    }

    pub fn unquote(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.unquote, obj]))
    }

    pub fn unquote_splicing(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.unquote_splicing, obj]))
    }

    pub fn list(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.list, obj]))
    }

    fn is_splicing(&self, value: Value) -> Result<Option<Value>, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(value) 
            && car == self.unquote_splicing
        {
            let (cadr, _) = self.to_pair(cdr)?;
            Ok(Some(cadr))
        } else {
            Ok(None)
        }
    }

    pub fn expand_quasiquote(&self, expr: Value) -> Result<Value, SchemeError> {
        match expr {
            Value::Object(id) => {
                let obj = {
                    self.heap.borrow().get(id).clone()
                };
                match obj {
                    HeapObject::Pair(car, cdr) if car == self.unquote => {
                        self.to_car(cdr)
                    },
                    HeapObject::Pair(..) => {
                        let mut p = expr;
                        let mut args = vec![self.append];
                        loop {
                            if let Some((car, cdr)) = self.is_pair(p) {
                                if let Some(spliced) = self.is_splicing(car)? {
                                    args.push(spliced)
                                } else {
                                    args.push(self.list(self.expand_quasiquote(car)?)?);
                                }
                                p = cdr;
                            } else if p == Value::Nil {
                                let mut heap = self.heap.borrow_mut();
                                return Ok(heap.alloc_list(&args));
                            } else {
                                let mut heap = self.heap.borrow_mut();
                                return Ok(heap.alloc_list_with_cdr(&args, p));
                            }
                        }
                        
                    },
                    _ => self.quote(expr)
                }
            },
            _ => self.quote(expr)
        }
    }

    fn expand_macro(&self, func: Value, args: Value) -> Result<Value, SchemeError> {
        let args = self.fold_list(
            args,
            Vec::new(), 
            |mut acc, arg| {
                acc.push(self.expand(arg)?);
                Ok(acc)
            });
        let expansion = match func.apply(self, self.env.clone(), args?)? {
            EvalResult::Done(value) =>  value,
            EvalResult::Continuation(next_env, next_expr) => {
                self.eval(next_env, next_expr)?
            }
        };
        Ok(expansion)
    }

    fn get_macro(&self, id: GcId) -> Option<Value> {
        // This function's purpose is to limit the scope of env borrowing.
        self.env.borrow().macros.get(&id).cloned()
    }

    pub fn expand(&self, expr: Value) -> Result<Value, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(expr) {
            if let Value::Object(id) = car && id == 8 {
                Ok(expr)
            } else if let Value::Object(id) = car
                && let Some(func) = self.get_macro(id) 
            {
                Ok(self.expand(self.expand_macro(func, cdr)?)?)
            } else {
                let mut updated = false;
                let items = self.fold_list(
                    expr, vec![], |mut acc, item| {
                        let expansion = self.expand(item)?;
                        updated = updated || expansion != item;
                        acc.push(expansion);
                        Ok(acc)
                    });
                if updated {
                    let expansion = {
                        let mut heap = self.heap.borrow_mut();
                        heap.alloc_list(&items?).clone()
                    };
                    Ok(expansion)
                } else {
                    Ok(expr)
                }
            }
        } else {
            Ok(expr)
        }
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Value, SchemeError> {
        let path = path.as_ref(); // Convert P into a &Path
        let content = std::fs::read_to_string(path)
            .map_err(|e| SchemeError::FileNotFound(e.to_string()))?;
    
        let mut parser = Parser::new(content.as_bytes());
        let mut retval = Value::Nil;
        while let Ok(expr) = parser.read(self) {
            if matches!(expr, Value::Nil) {
                break;
            }
            retval = self.expand(expr)?;
            retval = self.eval_full(self.env.clone(), retval)?;
        }
        Ok(retval)
    }

}

fn primitive_eval(interp: &Interp, env: Rc<RefCell<Env>>, args: &[Value])  
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    Ok(EvalResult::Done(interp.eval(env, args[0])?))
}

fn primitive_apply(interp: &Interp, env: Rc<RefCell<Env>>, args: &[Value])  
    -> Result<EvalResult, SchemeError> 
{
    use crate::heap::Apply;
    check_min_arity!(args, 2);
    let func = args[0];
    func.apply(interp, env, args[1..].to_vec())
}

fn primitive_expand(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])  -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(interp.expand(args[0])?))
}

fn primitive_equal(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])  
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 2);
    Ok(EvalResult::Done(Value::Boolean(args[0].is_equal(interp, &args[1]))))
}

fn primitive_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])  
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 2);
    Ok(EvalResult::Done(Value::Boolean(args[0] == args[1])))
}

fn primitive_error(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])  -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?.to_string();
    Err(SchemeError::UserError(string))
}

fn primitive_symbol_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])  -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_symbol(args[0]).is_some())))
}


fn primitive_add(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let sum = nums.into_iter()
        .fold(Number::Int(0), |acc, n| acc  + n);
    Ok(EvalResult::Done(Value::Number(sum)))
}

fn primitive_sub(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
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
    Ok(EvalResult::Done(Value::Number(sub)))
}

fn primitive_div(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
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
    Ok(EvalResult::Done(Value::Number(div)))
}


fn primitive_mul(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let mul = nums.into_iter()
        .fold(Number::Int(1), |acc, n| acc * n);
    Ok(EvalResult::Done(Value::Number(mul)))
}

fn primitive_rem(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Number(*a % *b)))
}

fn primitive_quit(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, exit_code: Number);
    match i32::try_from(*exit_code) {
        Ok(code) => process::exit(code),
        Err(_) => Err(SchemeError::OverflowError(format!(
            "Overflow while converting {} to i32", exit_code)
        ))
    }

}

fn primitive_number_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Boolean(a == b)))
}

fn primitive_number_lt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Boolean(a < b)))
}

fn primitive_number_lte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Boolean(a <= b)))
}

fn primitive_number_gt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Boolean(a > b)))
}

fn primitive_number_gte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    Ok(EvalResult::Done(Value::Boolean(a >= b)))
}

fn primitive_number_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_number(args[0]).is_some())))
}

fn primitive_integer_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_integer(args[0]).is_some())))
}

fn primitive_float_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_float(args[0]).is_some())))
}

fn primitive_number_max(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "max expects at least one arg.".to_string()));
    }
    let init = nums[0];
    let ret = nums.into_iter()
        .fold(init, |a, b| if a > b { a } else { b });
    Ok(EvalResult::Done(Value::Number(ret)))
}

fn primitive_number_min(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "min expects at least one arg.".to_string()));
    }
    let init = nums[0];
    let ret = nums.into_iter()
        .fold(init, |a, b| if a < b { a } else { b });
    Ok(EvalResult::Done(Value::Number(ret)))
}

fn primitive_list(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    if args.is_empty() {
        Ok(EvalResult::Done(Value::Nil))
    } else {
        Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_list(args)))
    }
}

fn primitive_append(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value])-> Result<EvalResult, SchemeError> {
    let mut retval = Value::Nil;
    let mut prev_cdr = Value::Nil;
    for (i, arg) in args.iter().enumerate() {
        if i == args.len() - 1 {
            if matches!(prev_cdr, Value::Nil) {
                retval = *arg; 
            } else {
                debug_assert!(matches!(retval, Value::Object(_)));
                let mut heap = interp.heap.borrow_mut();
                heap.setcdr(interp.to_object(prev_cdr)?, *arg)?;
            }
        } else {
            let mut p = *arg;
            while let Ok((car, cdr)) = interp.to_pair(p) {
                let mut heap = interp.heap.borrow_mut();
                if matches!(retval, Value::Nil) {
                    retval = heap.alloc_pair(car, Value::Nil);
                    prev_cdr = retval;
                } else {
                    let next = heap.alloc_pair(car, Value::Nil);
                    heap.setcdr(interp.to_object(prev_cdr)?, next)?;
                    prev_cdr = next;
                }
                p = cdr;
            }
            if ! matches!(p, Value::Nil) {
                return Err(SchemeError::TypeError(format!(
                    "Expected Nil, got a {}.", p.type_name()
                )))
            }
        }
    }
    Ok(EvalResult::Done(retval))
}

fn primitive_length(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let mut length = 0;
    if ! matches!(args[0], Value::Nil) {
        let (_, mut cdr) = interp.to_pair(args[0])?;
        loop {
            length += 1;
            if matches!(cdr, Value::Nil) { break; }
            (_, cdr) = interp.to_pair(cdr)?;
        }
    }
    Ok(EvalResult::Done(Value::Number(Number::Int(length))))
}

fn primitive_list_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_nil(args[0]) || interp.is_list(args[0]))))
}

fn primitive_null_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_null(args[0]))))
}

fn primitive_list_cons(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let mut heap = interp.heap.borrow_mut();
    Ok(EvalResult::Done(heap.alloc_pair(args[0], args[1])))
}

fn primitive_list_car(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let (car, _) = interp.to_pair(args[0])?;
    Ok(EvalResult::Done(car))
}

fn primitive_list_cdr(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let (_, cdr) = interp.to_pair(args[0])?;
    Ok(EvalResult::Done(cdr))
}

fn primitive_vector_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_vector(args[0]).is_some())))
}

fn primitive_make_vector(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_min_arity!(args, 1);
    let size = interp.to_integer(args[0])?;
    let mut fill_value = Value::Number(Number::Int(0));
    if args.len() == 2 {
        fill_value = args[1];
    }    
    let mut heap = interp.heap.borrow_mut();
    let data = vec![fill_value; size as usize];
    Ok(EvalResult::Done(heap.alloc_vector(&data)))
}

fn primitive_vector(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    let mut heap = interp.heap.borrow_mut();
    Ok(EvalResult::Done(heap.alloc_vector(args)))
}

fn primitive_vector_length(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let vector = interp.to_vector(args[0])?;
    Ok(EvalResult::Done(Value::Number(Number::Int(vector.data.borrow().len() as i64))))
}

fn primitive_vector_ref(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 2);
    let vector = interp.to_vector(args[0])?;
    let data = vector.data.borrow();
    let index = interp.to_integer(args[1])?;
    if index >= 0 && index < data.len() as i64 {
        Ok(EvalResult::Done(data[index as usize]))
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not within [0, {}[", index, data.len()
        )))
    }   
}

fn primitive_vector_set(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 3);
    let vector = interp.to_vector(args[0])?;
    let mut data = vector.data.borrow_mut();
    let index = interp.to_integer(args[1])?;
    if index >= 0 && index < data.len() as i64 {
        data[index as usize] = args[2];
        Ok(EvalResult::Done(args[2]))
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not within [0, {}[", index, data.len()
        )))
    }   
}

fn primitive_vector_to_list(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let items: Vec<Value> = {
        let vector = interp.to_vector(args[0])?;
        let data = vector.data.borrow();
        data.clone()
    };
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_list(&items)))
}

fn primitive_list_to_vector(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, _id: Object);
    let items = interp.fold_list(
        args[0], vec![], |mut acc, item| {
            acc.push(item);
            Ok(acc)
        }
    )?;
    let mut heap = interp.heap.borrow_mut();
    Ok(EvalResult::Done(heap.alloc_vector(&items)))
}

fn primitive_vector_fill(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let vector = interp.to_vector(args[0])?;
    let mut data = vector.data.borrow_mut();
    data.fill(args[1]);
    Ok(EvalResult::Done(args[1]))
}

fn primitive_char_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_char(args[0]).is_some())))
}

fn primitive_char_alphabetic_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Boolean((*ch as char).is_alphabetic())))
}

fn primitive_char_numeric_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Boolean((*ch as char).is_digit(10))))
}

fn primitive_char_whitespace_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Boolean(*ch == 9 || *ch == 10 || *ch == 32)))
}

fn primitive_char_upper_case_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Boolean((*ch as char).is_uppercase())))
}

fn primitive_char_lower_case_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Boolean((*ch as char).is_lowercase())))
}

fn primitive_char_to_integer(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Number(Number::Int(*ch as i64))))
}

fn primitive_integer_to_char(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let byte = interp.to_integer(args[0])?;
    Ok(EvalResult::Done(Value::Char(byte as u8)))
}

fn primitive_char_upcase(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Char((*ch as char).to_ascii_uppercase() as u8)))
}

fn primitive_char_downcase(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    Ok(EvalResult::Done(Value::Char((*ch as char).to_ascii_lowercase() as u8)))
}

fn primitive_char_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1 == ch2)))
}

fn primitive_char_lt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1 < ch2)))
}

fn primitive_char_lte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1 <= ch2)))
}

fn primitive_char_gt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1 > ch2)))
}

fn primitive_char_gte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1 >= ch2)))
}

fn primitive_char_ci_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1.to_ascii_lowercase() == ch2.to_ascii_lowercase())))
}

fn primitive_char_ci_lt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1.to_ascii_lowercase() < ch2.to_ascii_lowercase())))
}

fn primitive_char_ci_lte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1.to_ascii_lowercase() <= ch2.to_ascii_lowercase())))
}

fn primitive_char_ci_gt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1.to_ascii_lowercase() > ch2.to_ascii_lowercase())))
}

fn primitive_char_ci_gte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    Ok(EvalResult::Done(Value::Boolean(ch1.to_ascii_lowercase() >= ch2.to_ascii_lowercase())))
}

fn primitive_string_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(interp.is_string(args[0]).is_some())))
}

fn primitive_make_string(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let mut fill_char = 32 as char;
    check_min_arity!(args, 1);
    let count = interp.to_integer(args[0])?;
    if args.len() > 1 {
        fill_char = interp.to_char(args[1])?;
    }
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_string(
        fill_char.to_string().repeat(count as usize))
    ))
}

fn primitive_string(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let mut buf = String::new();
    for arg in args {
        let ch = interp.to_char(*arg)?;
        buf.push(ch);
    }
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_string(buf)))
}

fn primitive_string_to_list(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, _id: Object);
    let chars:Vec<Value> = {
        let string = interp.to_string(args[0])?;
        string.chars().map(
            |ch| Value::Char(ch as u8)
        ).collect()
    };
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_list(&chars)))
}

fn primitive_list_to_string(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, _id: Object);
    let chars = interp.fold_list(
        args[0], String::new(), |mut acc, item| {
            let ch = interp.to_char(item)?;
            acc.push(ch);
            Ok(acc)
        })?;
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_string(&chars)))
}

fn primitive_string_length(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?;
    Ok(EvalResult::Done(Value::Number(Number::Int(string.len() as i64))))
}

fn primitive_string_ref(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let string = interp.to_string(args[0])?;
    let index = interp.to_integer(args[1])?;
    if index >= 0 && index < (string.len() as i64) 
        && let Some(ch) = string.chars().nth(index as usize) {
        Ok(EvalResult::Done(Value::Char(ch as u8)))
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not in 0..{}", index, string.len()
        )))
    }
}

fn primitive_string_set(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 3);
    let mut string = interp.to_string_mut(args[0])?;
    let index = interp.to_integer(args[1])?;
    let value = interp.to_char(args[2])?;
    if index >= 0 && index < (string.len() as i64) {
        // TODO This is really horrible!
        string.remove(index as usize);
        string.insert(index as usize, value as char);
        Ok(EvalResult::Done(args[0]))
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not in 0..{}", index, string.len()
        )))
    }
}

fn primitive_string_eq(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa == sb)))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_lt(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa < sb)))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_gt(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa > sb)))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_lte(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa <= sb)))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_gte(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa >= sb)))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_ci_eq(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa.to_ascii_lowercase() == sb.to_ascii_lowercase())))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_ci_lt(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa.to_ascii_lowercase() < sb.to_ascii_lowercase())))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}
fn primitive_string_ci_lte(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa.to_ascii_lowercase() <= sb.to_ascii_lowercase())))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}
fn primitive_string_ci_gt(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa.to_ascii_lowercase() > sb.to_ascii_lowercase())))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}
fn primitive_string_ci_gte(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            Ok(EvalResult::Done(Value::Boolean(sa.to_ascii_lowercase() >= sb.to_ascii_lowercase())))
        },
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}", xa.type_name(), xb.type_name()
        )))
    }
}

fn primitive_string_append(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    let mut buf = String::new();
    for arg in args {
        let string = interp.to_string(*arg)?;
        buf.push_str(&string);
    }
    let mut heap = interp.heap.borrow_mut();
    Ok(EvalResult::Done(heap.alloc_string(buf)))
}

fn primitive_substring(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 3);
    let string = interp.to_string(args[0])?.to_string();
    let start_index = interp.to_integer(args[1])?;
    let end_index = interp.to_integer(args[2])?;
    if start_index < 0 || start_index > string.len() as i64 {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Start index {} is not within 0..{}", start_index, string.len()
        )))
    } else if end_index < start_index || end_index > string.len() as i64 {
        Err(SchemeError::IndexOutOfBounds(format!(
            "End index {} is not within {}..{}", end_index, start_index, string.len()
        )))
    } else {
        Ok(EvalResult::Done(
            interp.heap.borrow_mut()
                .alloc_string(&string[start_index as usize..end_index as usize])
        ))
    }
}

fn primitive_string_copy(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?.to_string();
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_string(string)))
}

fn primitive_string_fill(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 2);
    let mut string = interp.to_string_mut(args[0])?.to_string();
    let ch = interp.to_char(args[1])?;
    // TODO Again this is really ugly!
    let count = string.chars().count();
    string.clear();
    for _ in 0..count {
        string.push(ch);
    }
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_string(string)))
}

/**
 * IO primitives.
 */
fn primitive_open_input_file(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let filename = interp.to_string(args[0])?.to_string();
    let file = File::open(&filename).map_err(|_| {
        SchemeError::FileNotFound(format!(
            "Can't open file {}", filename))
    })?;
    let reader = BufReader::new(file);
    let boxed_reader: Box<dyn BufRead> = Box::new(reader);
    let input = Rc::new(RefCell::new(Some(boxed_reader)));
    Ok(EvalResult::Done(interp.heap.borrow_mut().alloc_input_port(input)))
}

fn primitive_close_input_port(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let input = interp.to_input_port(args[0])?;
    let reader = input.borrow_mut().take();
    if ! reader.is_none() {
        println!("File closed.");
    } 
    Ok(EvalResult::Done(Value::Nil))
}

fn primitive_read_char(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let input = interp.to_input_port(args[0])?;
    let mut borrow = input.borrow_mut();
    if let Some(ref mut reader) = *borrow {
        let mut buf = [0u8; 1];
        match reader.read_exact(&mut buf) {
            Ok(_) => {
                Ok(EvalResult::Done(Value::Char(buf[0])))
            },
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                Ok(EvalResult::Done(Value::Eof))
            },
            Err(e) => {
                Err(SchemeError::IOError(format!("Read error {}", e)))
            }
        }
    } else {
        Err(SchemeError::IOError(format!(
            "Attempt to read from closed input port."
        )))
    }
}

fn primitive_eof_object(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    Ok(EvalResult::Done(Value::Boolean(args[0] == Value::Eof)))
}

fn primitive_gc(interp: &Interp, env: Rc<RefCell<Env>>, _args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    // Marks all reachable objects from the environment and the heap's symbols.
    let len = interp.heap.borrow().len();
    let mut marks = MarkSet::new(len);
    env.borrow().mark(interp, &mut marks);
    interp.heap.borrow().mark(interp, &mut marks);
    
    // Collects all unreachable objects lying in the heap.
    let mut heap = interp.heap.borrow_mut();
    let collected = heap.collect(&marks);

    println!("gc: marked {} /{} objects, collected {}.", marks.count(), len, collected);   

    Ok(EvalResult::Done(Value::Nil))
}

fn primitive_debug(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", interp.display(*arg))
    }
    println!();
    Ok(EvalResult::Done(Value::Boolean(true)))
}

fn primitive_load(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    // TODO SyntaxError don't seem to come all theway back here.
    let mut retval = Value::Nil;
    for arg in args {
        let filename = interp.to_string(*arg)?.to_string();
        retval = interp.load(&filename)?;
    }
    Ok(EvalResult::Done(retval))
}

fn primitive_heap_stats(interp: &Interp, _env: Rc<RefCell<Env>>, _args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    let stats = interp.heap.borrow().stats();
    println!("Total slots: {}", stats.total_slots);
    println!(" Live slots: {}", stats.live_slots);
    println!(" Free slots: {}", stats.free_slots);
    println!("  Next slot: {}", stats.next_slot);
    println!("    Symbols: {}", stats.symbol_count);
    Ok(EvalResult::Done(Value::Nil))
}