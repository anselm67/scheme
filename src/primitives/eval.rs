use std::{cell::RefCell, rc::Rc};

use crate::{
    env::Env,
    interp::Interp,
    types::{EvalResult, SchemeError, Value},
};

fn primitive_eval(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(interp.eval(env, args[0])?)
}

fn primitive_apply(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    use crate::heap::Apply;
    check_min_arity!(args, 2);
    let func = args[0];
    let (last, firsts) = args[1..]
        .split_last()
        .ok_or(SchemeError::ArgCountError(format!(
            "Expected at least 2 args, got {}",
            args.len()
        )))?;
    let all_args = interp.fold_list(*last, firsts.to_vec(), |mut acc, arg| {
        acc.push(arg);
        Ok(acc)
    })?;
    func.apply(interp, env, all_args)
}

fn primitive_expand(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(interp.expand(args[0])?)
}

fn primitive_equal(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::done(Value::Boolean(args[0].is_equal(interp, &args[1])))
}

fn primitive_eq(
    _interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::done(Value::Boolean(args[0] == args[1]))
}

fn primitive_error(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?.to_string();
    Err(SchemeError::UserError(string))
}

fn primitive_with_exception_handler(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let handler = args[0];
    let thunk = args[1];
    match interp.apply(env.clone(), thunk, vec![]) {
        Ok(value) => EvalResult::done(value),
        Err(e) => {
            let (label, message) = e.get_infos();
            let string = interp
                .heap
                .borrow_mut()
                .alloc_string(format!("[{}]: {}", label, message));
            EvalResult::done(interp.apply(env.clone(), handler, vec![string])?)
        }
    }
}

fn primitive_procedure_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_procedure(args[0])))
}

fn primitive_closure_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_closure(args[0]).is_some()))
}

fn primitive_closure_body(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let body = {
        let closure = interp.to_closure(args[0])?;
        closure.get_body()
    };
    let mut heap = interp.heap.borrow_mut();
    EvalResult::done(heap.alloc_list(&body))
}

fn primitive_symbol_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_symbol(args[0]).is_some()))
}

pub fn register(interp: &Interp) {
    interp.define("#t", Value::Boolean(true));
    interp.define("#f", Value::Boolean(false));
    interp.define_primitive("eval", primitive_eval);
    interp.define_primitive("apply", primitive_apply);
    interp.define_primitive("expand", primitive_expand);
    interp.define_primitive("eq?", primitive_eq);
    interp.define_primitive("equal?", primitive_equal);
    interp.define_primitive("error", primitive_error);
    interp.define_primitive("with-exception-handler", primitive_with_exception_handler);
    interp.define_primitive("procedure?", primitive_procedure_p);
    interp.define_primitive("closure?", primitive_closure_p);
    interp.define_primitive("closure->body", primitive_closure_body);
    interp.define_primitive("symbol?", primitive_symbol_p);
}
