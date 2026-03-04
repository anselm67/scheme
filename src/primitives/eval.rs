use crate::{
    interp::Scheme,
    types::{EvalFuture, EvalResult, Number, SchemeError, Value},
};

fn primitive_eval<'a>(interp: &'a Scheme, env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        EvalResult::done(interp.eval(env, args[0]).await?)
    })
}

fn primitive_apply<'a>(interp: &'a Scheme, env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    use crate::heap::Apply;
    Box::pin(async move {
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
        func.apply(interp, env, all_args).await
    })
}

fn primitive_expand<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let expansion = interp.expand(args[0]).await?;
        EvalResult::done(expansion.value())
    })
}

fn primitive_equal(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::bool(args[0].is_equal(interp, &args[1]))
}

fn primitive_eq(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let value = match (args[0], args[1]) {
        (Value::Number(Number::Float(_)), Value::Number(Number::Int(_))) => false,
        (Value::Number(Number::Int(_)), Value::Number(Number::Float(_))) => false,
        (a, b) => a == b,
    };
    EvalResult::bool(value)
}

fn primitive_error(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?;
    Err(SchemeError::UserError(string.borrow().clone()))
}

fn primitive_with_exception_handler<'a>(
    interp: &'a Scheme,
    env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 2);
        let handler = args[0];
        let thunk = args[1];
        match interp.apply(env, thunk, vec![]).await {
            Ok(value) => EvalResult::done(value),
            Err(e) => {
                let (label, message) = e.get_infos();
                let string = interp.alloc_string(format!("[{}]: {}", label, message));
                EvalResult::done(interp.apply(env, handler, vec![string.value()]).await?)
            }
        }
    })
}

fn primitive_procedure_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(interp.is_procedure(args[0]))
}

fn primitive_closure_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(interp.is_closure(args[0]).is_some())
}

fn primitive_closure_body(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let body = {
        let closure = interp.to_closure(args[0])?;
        closure.get_body()
    };
    EvalResult::done(interp.alloc_list(&body).value())
}

fn primitive_macro_body(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let id = interp.to_object(args[0])?;
    if let Some(value) = interp.get_macro(id) {
        EvalResult::done(value)
    } else {
        Err(SchemeError::UnboundVariable(format!("macro not found.")))
    }
}

fn primitive_symbol_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(interp.is_symbol(args[0]).is_some())
}

fn primitive_symbol_to_string(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let name = interp.to_symbol_name(args[0])?;
    let string = interp.alloc_string(name);
    EvalResult::done(string.value())
}

fn primitive_string_to_symbol(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let name = interp.to_string(args[0])?;
    let name = name.borrow();
    let (_, symbol) = interp.intern_symbol(&name);
    EvalResult::done(symbol.value())
}

pub fn register(interp: &Scheme) {
    interp.define_from_string("#t", Value::Boolean(true));
    interp.define_from_string("#f", Value::Boolean(false));
    interp.define_async_primitive("eval", primitive_eval);
    interp.define_async_primitive("apply", primitive_apply);
    interp.define_async_primitive("expand", primitive_expand);
    interp.define_primitive("eq?", primitive_eq);
    interp.define_primitive("equal?", primitive_equal);
    interp.define_primitive("error", primitive_error);
    interp.define_async_primitive("with-exception-handler", primitive_with_exception_handler);
    interp.define_primitive("procedure?", primitive_procedure_p);
    interp.define_primitive("closure?", primitive_closure_p);
    interp.define_primitive("closure->body", primitive_closure_body);
    interp.define_primitive("macro->body", primitive_macro_body);
    interp.define_primitive("symbol?", primitive_symbol_p);
    interp.define_primitive("symbol->string", primitive_symbol_to_string);
    interp.define_primitive("string->symbol", primitive_string_to_symbol);
}
