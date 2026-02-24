use crate::{
    interp::Scheme,
    types::{EvalResult, Number, SchemeError, Value},
};

fn primitive_add(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let sum = nums.into_iter().fold(Number::Int(0), |acc, n| acc + n);
    EvalResult::done(Value::Number(sum))
}

fn primitive_sub(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "- expects at least one arg.".to_string(),
        ));
    }

    let mut iter = nums.into_iter();
    let init = iter.next().unwrap();
    let sub = if let None = iter.clone().next() {
        -init
    } else {
        iter.fold(init, |acc, n| acc - n)
    };
    EvalResult::done(Value::Number(sub))
}

fn primitive_div(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "- expects at least one arg.".to_string(),
        ));
    }

    let mut iter = nums.into_iter();
    let init = iter.next().unwrap();
    let div = if let None = iter.clone().next() {
        Number::Float(1.0) / init
    } else {
        iter.fold(init, |acc, n| acc / n)
    };
    EvalResult::done(Value::Number(div))
}

fn primitive_mul(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let mul = nums.into_iter().fold(Number::Int(1), |acc, n| acc * n);
    EvalResult::done(Value::Number(mul))
}

fn primitive_rem(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Number(*a % *b))
}

fn primitive_number_eq(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Boolean(a == b))
}

fn primitive_number_lt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Boolean(a < b))
}

fn primitive_number_lte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Boolean(a <= b))
}

fn primitive_number_gt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Boolean(a > b))
}

fn primitive_number_gte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::done(Value::Boolean(a >= b))
}

fn primitive_number_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_number(args[0]).is_some()))
}

fn primitive_integer_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_integer(args[0]).is_some()))
}

fn primitive_float_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_float(args[0]).is_some()))
}

fn primitive_number_max(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "max expects at least one arg.".to_string(),
        ));
    }
    let init = nums[0];
    let ret = nums
        .into_iter()
        .fold(init, |a, b| if a > b { a } else { b });
    EvalResult::done(Value::Number(ret))
}

fn primitive_number_min(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    if nums.is_empty() {
        return Err(SchemeError::ArgCountError(
            "min expects at least one arg.".to_string(),
        ));
    }
    let init = nums[0];
    let ret = nums
        .into_iter()
        .fold(init, |a, b| if a < b { a } else { b });
    EvalResult::done(Value::Number(ret))
}

fn primitive_sqt(interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let value = interp.to_number(args[0])?;

    EvalResult::done(Value::Number(value.sqrt()))
}

pub fn register(interp: &Scheme) {
    interp.define_primitive("number?", primitive_number_p);
    interp.define_primitive("integer?", primitive_integer_p);
    interp.define_primitive("float?", primitive_float_p);
    interp.define_primitive("+", primitive_add);
    interp.define_primitive("-", primitive_sub);
    interp.define_primitive("*", primitive_mul);
    interp.define_primitive("/", primitive_div);
    interp.define_primitive("%", primitive_rem);
    interp.define_primitive("=", primitive_number_eq);
    interp.define_primitive("<", primitive_number_lt);
    interp.define_primitive(">", primitive_number_gt);
    interp.define_primitive("<=", primitive_number_lte);
    interp.define_primitive(">=", primitive_number_gte);
    interp.define_primitive("max", primitive_number_max);
    interp.define_primitive("min", primitive_number_min);
    interp.define_primitive("sqt", primitive_sqt);
}
