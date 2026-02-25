use crate::{
    interp::Scheme,
    types::{EvalResult, Number, SchemeError, Value},
};

fn primitive_list(interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    if args.is_empty() {
        EvalResult::done(Value::Nil)
    } else {
        EvalResult::done(interp.alloc_list(args).value())
    }
}

fn primitive_append(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
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
                if matches!(retval, Value::Nil) {
                    // TODO This can't  be right.
                    retval = interp.alloc_pair(car, Value::Nil).value();
                    prev_cdr = retval;
                } else {
                    // TODO This can't  be right.
                    let next = interp.alloc_pair(car, Value::Nil).value();
                    interp.setcdr(interp.to_object(prev_cdr)?, next)?;
                    prev_cdr = next;
                }
                p = cdr;
            }
            if !matches!(p, Value::Nil) {
                return Err(SchemeError::TypeError(format!(
                    "Expected Nil, got a {}.",
                    p.type_name()
                )));
            }
        }
    }
    EvalResult::done(retval)
}

fn primitive_length(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let mut length = 0;
    if !matches!(args[0], Value::Nil) {
        let (_, mut cdr) = interp.to_pair(args[0])?;
        loop {
            length += 1;
            if matches!(cdr, Value::Nil) {
                break;
            }
            (_, cdr) = interp.to_pair(cdr)?;
        }
    }
    EvalResult::done(Value::Number(Number::Int(length)))
}

fn primitive_pair_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_pair(args[0]).is_some()))
}

fn primitive_list_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(
        interp.is_nil(args[0]) || interp.is_list(args[0]),
    ))
}

fn primitive_null_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_null(args[0])))
}

fn primitive_list_cons(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::done(interp.alloc_pair(args[0], args[1]).value())
}

fn primitive_list_car(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let (car, _) = interp.to_pair(args[0])?;
    EvalResult::done(car)
}

fn primitive_list_cdr(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let (_, cdr) = interp.to_pair(args[0])?;
    EvalResult::done(cdr)
}

pub fn register(interp: &Scheme) {
    interp.define_primitive("list", primitive_list);
    interp.define_primitive("append", primitive_append);
    interp.define_primitive("length", primitive_length);
    interp.define_primitive("pair?", primitive_pair_p);
    interp.define_primitive("list?", primitive_list_p);
    interp.define_primitive("null?", primitive_null_p);
    interp.define_primitive("cons", primitive_list_cons);
    interp.define_primitive("car", primitive_list_car);
    interp.define_primitive("cdr", primitive_list_cdr);
}
