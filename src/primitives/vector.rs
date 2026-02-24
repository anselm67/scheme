use crate::{
    interp::Scheme,
    types::{EvalResult, Number, SchemeError, Value},
};

fn primitive_vector_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_vector(args[0]).is_some()))
}

fn primitive_make_vector(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_min_arity!(args, 1);
    let size = interp.to_integer(args[0])?;
    let mut fill_value = Value::Number(Number::Int(0));
    if args.len() == 2 {
        fill_value = args[1];
    }
    let data = vec![fill_value; size as usize];
    EvalResult::done(interp.alloc_vector(&data).value())
}

fn primitive_vector(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    EvalResult::done(interp.alloc_vector(args).value())
}

fn primitive_vector_length(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let vector = interp.to_vector(args[0])?;
    EvalResult::done(Value::Number(
        Number::Int(vector.data.borrow().len() as i64),
    ))
}

fn primitive_vector_ref(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let vector = interp.to_vector(args[0])?;
    let data = vector.data.borrow();
    let index = interp.to_integer(args[1])?;
    if index >= 0 && index < data.len() as i64 {
        EvalResult::done(data[index as usize])
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not within [0, {}[",
            index,
            data.len()
        )))
    }
}

fn primitive_vector_set(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 3);
    let vector = interp.to_vector(args[0])?;
    let mut data = vector.data.borrow_mut();
    let index = interp.to_integer(args[1])?;
    if index >= 0 && index < data.len() as i64 {
        data[index as usize] = args[2];
        EvalResult::done(args[2])
    } else {
        Err(SchemeError::IndexOutOfBounds(format!(
            "Index {} is not within [0, {}[",
            index,
            data.len()
        )))
    }
}

fn primitive_vector_to_list(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let items: Vec<Value> = {
        let vector = interp.to_vector(args[0])?;
        let data = vector.data.borrow();
        data.clone()
    };
    EvalResult::done(interp.alloc_list(&items).value())
}

fn primitive_list_to_vector(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, _id: Object);
    let items = interp.fold_list(args[0], vec![], |mut acc, item| {
        acc.push(item);
        Ok(acc)
    })?;
    EvalResult::done(interp.alloc_vector(&items).value())
}

fn primitive_vector_fill(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let vector = interp.to_vector(args[0])?;
    let mut data = vector.data.borrow_mut();
    data.fill(args[1]);
    EvalResult::done(args[1])
}

pub fn register(interp: &Scheme) {
    interp.define_primitive("vector?", primitive_vector_p);
    interp.define_primitive("make-vector", primitive_make_vector);
    interp.define_primitive("vector", primitive_vector);
    interp.define_primitive("vector-length", primitive_vector_length);
    interp.define_primitive("vector-ref", primitive_vector_ref);
    interp.define_primitive("vector-set!", primitive_vector_set);
    interp.define_primitive("vector->list", primitive_vector_to_list);
    interp.define_primitive("list->vector", primitive_list_to_vector);
    interp.define_primitive("vector-fill!", primitive_vector_fill);
}
