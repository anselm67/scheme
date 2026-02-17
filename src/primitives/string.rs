use std::{cell::RefCell, rc::Rc};

use crate::{
    env::Env,
    heap::HeapObject,
    interp::Interp,
    types::{EvalResult, Number, SchemeError, Value},
};

fn primitive_string_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_string(args[0]).is_some()))
}

fn primitive_make_string(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let mut fill_char = 32 as char;
    check_min_arity!(args, 1);
    let count = interp.to_integer(args[0])?;
    if args.len() > 1 {
        fill_char = interp.to_char(args[1])?;
    }
    EvalResult::done(
        interp
            .heap
            .borrow_mut()
            .alloc_string(fill_char.to_string().repeat(count as usize)),
    )
}

fn primitive_string(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let mut buf = String::new();
    for arg in args {
        let ch = interp.to_char(*arg)?;
        buf.push(ch);
    }
    EvalResult::done(interp.heap.borrow_mut().alloc_string(buf))
}

fn primitive_string_to_list(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, _id: Object);
    let chars: Vec<Value> = {
        let string = interp.to_string(args[0])?;
        string.chars().map(|ch| Value::Char(ch as u8)).collect()
    };
    EvalResult::done(interp.heap.borrow_mut().alloc_list(&chars))
}

fn primitive_list_to_string(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, _id: Object);
    let chars = interp.fold_list(args[0], String::new(), |mut acc, item| {
        let ch = interp.to_char(item)?;
        acc.push(ch);
        Ok(acc)
    })?;
    EvalResult::done(interp.heap.borrow_mut().alloc_string(&chars))
}

fn primitive_string_length(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?;
    EvalResult::done(Value::Number(Number::Int(string.len() as i64)))
}

fn primitive_string_ref(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let string = interp.to_string(args[0])?;
    let index = interp.to_integer(args[1])?;
    if index >= 0
        && index < (string.len() as i64)
        && let Some(ch) = string.chars().nth(index as usize)
    {
        EvalResult::done(Value::Char(ch as u8))
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not in 0..{}",
            index,
            string.len()
        )))
    }
}

fn primitive_string_set(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 3);
    let mut string = interp.to_string_mut(args[0])?;
    let index = interp.to_integer(args[1])?;
    let value = interp.to_char(args[2])?;
    if index >= 0 && index < (string.len() as i64) {
        // TODO This is really horrible!
        string.remove(index as usize);
        string.insert(index as usize, value as char);
        EvalResult::done(args[0])
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not in 0..{}",
            index,
            string.len()
        )))
    }
}

fn with_string<F>(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
    f: F,
) -> Result<EvalResult, SchemeError>
where
    F: FnOnce(&String, &String) -> bool, // Use the Fn trait
{
    extract_args!(args, 2, aid: Object, bid: Object);
    let heap = interp.heap.borrow();
    match (heap.get(*aid), heap.get(*bid)) {
        (HeapObject::String(sa), HeapObject::String(sb)) => {
            let result = f(sa, sb);
            EvalResult::done(Value::Boolean(result))
        }
        (xa, xb) => Err(SchemeError::TypeError(format!(
            "String comparion requires two String, got {} and {}",
            xa.type_name(),
            xb.type_name()
        ))),
    }
}

fn primitive_string_eq(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| a == b)
}

fn primitive_string_lt(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| a < b)
}

fn primitive_string_gt(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| a > b)
}

fn primitive_string_lte(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| a <= b)
}

fn primitive_string_gte(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| a >= b)
}

fn primitive_string_ci_eq(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| {
        a.to_lowercase() == b.to_lowercase()
    })
}

fn primitive_string_ci_lt(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| {
        a.to_lowercase() < b.to_lowercase()
    })
}

fn primitive_string_ci_lte(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| {
        a.to_lowercase() <= b.to_lowercase()
    })
}

fn primitive_string_ci_gt(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| {
        a.to_lowercase() > b.to_lowercase()
    })
}

fn primitive_string_ci_gte(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_string(interp, _env, args, |a, b| {
        a.to_lowercase() >= b.to_lowercase()
    })
}

fn primitive_string_append(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let mut buf = String::new();
    for arg in args {
        let string = interp.to_string(*arg)?;
        buf.push_str(&string);
    }
    let mut heap = interp.heap.borrow_mut();
    EvalResult::done(heap.alloc_string(buf))
}

fn primitive_substring(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 3);
    let string = interp.to_string(args[0])?.to_string();
    let start_index = interp.to_integer(args[1])?;
    let end_index = interp.to_integer(args[2])?;
    if start_index < 0 || start_index > string.len() as i64 {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Start index {} is not within 0..{}",
            start_index,
            string.len()
        )))
    } else if end_index < start_index || end_index > string.len() as i64 {
        Err(SchemeError::IndexOutOfBounds(format!(
            "End index {} is not within {}..{}",
            end_index,
            start_index,
            string.len()
        )))
    } else {
        EvalResult::done(
            interp
                .heap
                .borrow_mut()
                .alloc_string(&string[start_index as usize..end_index as usize]),
        )
    }
}

fn primitive_string_copy(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?.to_string();
    EvalResult::done(interp.heap.borrow_mut().alloc_string(string))
}

fn primitive_string_fill(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let mut string = interp.to_string_mut(args[0])?.to_string();
    let ch = interp.to_char(args[1])?;
    // TODO Again this is really ugly!
    let count = string.chars().count();
    string.clear();
    for _ in 0..count {
        string.push(ch);
    }
    EvalResult::done(interp.heap.borrow_mut().alloc_string(string))
}

pub fn register(interp: &Interp) {
    interp.define_primitive("string?", primitive_string_p);
    interp.define_primitive("make-string", primitive_make_string);
    interp.define_primitive("string", primitive_string);
    interp.define_primitive("string->list", primitive_string_to_list);
    interp.define_primitive("list->string", primitive_list_to_string);
    interp.define_primitive("string-length", primitive_string_length);
    interp.define_primitive("string-ref", primitive_string_ref);
    interp.define_primitive("string-set!", primitive_string_set);
    interp.define_primitive("string=?", primitive_string_eq);
    interp.define_primitive("string<?", primitive_string_lt);
    interp.define_primitive("string<=?", primitive_string_lte);
    interp.define_primitive("string>?", primitive_string_gt);
    interp.define_primitive("string>=?", primitive_string_gte);
    interp.define_primitive("string-ci=?", primitive_string_ci_eq);
    interp.define_primitive("string-ci<?", primitive_string_ci_lt);
    interp.define_primitive("string-ci<=?", primitive_string_ci_lte);
    interp.define_primitive("string-ci>?", primitive_string_ci_gt);
    interp.define_primitive("string-ci>=?", primitive_string_ci_gte);
    interp.define_primitive("string-append", primitive_string_append);
    interp.define_primitive("substring", primitive_substring);
    interp.define_primitive("string-copy", primitive_string_copy);
    interp.define_primitive("string-fill!", primitive_string_fill);
}
