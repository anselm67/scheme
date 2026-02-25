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

fn primitive_quotient(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let a = interp.to_integer(args[0])?;
    let b = interp.to_integer(args[1])?;
    EvalResult::done(Value::Number(Number::Int(a / b)))
}

fn primitive_modulo(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let a = interp.to_integer(args[0])?;
    let b = interp.to_integer(args[1])?;
    EvalResult::done(Value::Number(Number::Int(a.rem_euclid(b))))
}

fn with_numbers<F>(interp: &Scheme, args: &[Value], cmp: F) -> Result<EvalResult, SchemeError>
where
    F: Fn(Number, Number) -> bool,
{
    check_min_arity!(args, 2);
    let mut a = interp.to_number(args[0])?;
    for arg in &args[1..] {
        let b = interp.to_number(*arg)?;
        if !cmp(a, b) {
            return EvalResult::done(Value::Boolean(false));
        }
        a = b;
    }
    return EvalResult::done(Value::Boolean(true));
}
fn primitive_number_eq(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_numbers(interp, args, |a, b| a == b)
}

fn primitive_number_lt(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_numbers(interp, args, |a, b| a < b)
}

fn primitive_number_lte(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_numbers(interp, args, |a, b| a <= b)
}

fn primitive_number_gt(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_numbers(interp, args, |a, b| a > b)
}

fn primitive_number_gte(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    with_numbers(interp, args, |a, b| a >= b)
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

fn gcd_two(a: i64, b: i64) -> i64 {
    let mut a = a.abs();
    let mut b = b.abs();
    while b != 0 {
        a %= b;
        std::mem::swap(&mut a, &mut b);
    }
    a
}

fn primitive_gcd(interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let mut result = 0;
    for arg in args {
        let n = interp.to_integer(*arg)?;
        result = gcd_two(result, n);
    }
    EvalResult::done(Value::Number(Number::Int(result)))
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
    interp.define_primitive("quotient", primitive_quotient);
    interp.define_primitive("modulo", primitive_modulo);
    interp.define_primitive("=", primitive_number_eq);
    interp.define_primitive("<", primitive_number_lt);
    interp.define_primitive(">", primitive_number_gt);
    interp.define_primitive("<=", primitive_number_lte);
    interp.define_primitive(">=", primitive_number_gte);
    interp.define_primitive("max", primitive_number_max);
    interp.define_primitive("min", primitive_number_min);
    interp.define_primitive("gcd", primitive_gcd);
    interp.define_primitive("sqt", primitive_sqt);
}
