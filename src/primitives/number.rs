use crate::{
    interp::Scheme,
    types::{EvalResult, Number, SchemeError, Value},
};

fn primitive_add(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let sum = nums.into_iter().fold(Number::Int(0), |acc, n| acc + n);
    EvalResult::number(sum)
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
    EvalResult::number(sub)
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
    EvalResult::number(div)
}

fn primitive_mul(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    let nums = all_of_type!(args, Value::Number, "Number");
    let mul = nums.into_iter().fold(Number::Int(1), |acc, n| acc * n);
    EvalResult::number(mul)
}

fn primitive_rem(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, a: Number, b: Number);
    EvalResult::number(*a % *b)
}

fn primitive_quotient(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let a = interp.to_integer(args[0])?;
    let b = interp.to_integer(args[1])?;
    EvalResult::int(a / b)
}

fn primitive_modulo(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let a = interp.to_integer(args[0])?;
    let b = interp.to_integer(args[1])?;
    EvalResult::int(a.rem_euclid(b))
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
    return EvalResult::bool(true);
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
    EvalResult::bool(interp.is_integer(args[0]).is_some())
}

fn primitive_float_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(interp.is_float(args[0]).is_some())
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
    EvalResult::number(ret)
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
    EvalResult::number(ret)
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
    EvalResult::int(result)
}

fn primitive_number_to_string(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity_range!(args, 1, 2);
    let number = interp.to_number(args[0])?;
    let radix = if args.len() == 2 {
        interp.to_integer(args[1])? as u32
    } else {
        10
    };
    let string = match number {
        Number::Int(value) => match radix {
            2 => format!("{:b}", value),
            8 => format!("{:o}", value),
            10 => format!("{}", value),
            16 => format!("{:x}", value),
            _ => Err(SchemeError::UnsupportedError(format!(
                "Radix {radix} isn't suported, supported radixes are 2, 8, 10 and 16."
            )))?,
        },
        Number::Float(value) => {
            if radix != 10 {
                Err(SchemeError::UnsupportedError(format!(
                    "Radix {radix} isn't supported, only supported radix for floats is 10."
                )))?
            } else {
                format!("{}", value)
            }
        }
    };
    EvalResult::done(interp.alloc_string(&string).value())
}

fn primitive_exact_to_inexact(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, n: Number);
    match n {
        Number::Int(i) => EvalResult::int(*i),
        _ => EvalResult::done(args[0]),
    }
}

fn primitive_inexact_to_exact(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, n: Number);
    match n {
        Number::Float(f) => EvalResult::int(f.round() as i64),
        _ => EvalResult::done(args[0]),
    }
}

fn primitive_round(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, n: Number);
    match n {
        Number::Float(f) => EvalResult::float(f.round()),
        _ => EvalResult::done(args[0]),
    }
}

fn primitive_sqt(interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let value = interp.to_number(args[0])?;

    EvalResult::number(value.sqrt())
}

fn primitive_log(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, number: Number);
    EvalResult::number(number.log())
}

fn primitive_floor(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, number: Number);
    EvalResult::number(number.floor())
}

fn primitive_expt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, base: Number, exp: Number);
    EvalResult::number(base.expt(*exp))
}

fn primitive_sin(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, number: Number);
    EvalResult::number(number.sin())
}

fn primitive_cos(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, number: Number);
    EvalResult::number(number.cos())
}

fn primitive_tan(_interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, number: Number);
    EvalResult::number(number.tan())
}

fn primitive_atan(interp: &Scheme, _env: Value, args: &[Value]) -> Result<EvalResult, SchemeError> {
    check_arity_range!(args, 1, 2);
    let first = interp.to_number(args[0])?;
    if args.len() == 1 {
        EvalResult::number(first.atan())
    } else {
        let second = interp.to_number(args[1])?;
        EvalResult::number(first.atan2(second))
    }
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
    interp.define_primitive("number->string", primitive_number_to_string);
    interp.define_primitive("exact->inexact", primitive_exact_to_inexact);
    interp.define_primitive("inexact->exact", primitive_inexact_to_exact);
    interp.define_primitive("round", primitive_round);
    interp.define_primitive("floor", primitive_floor);
    interp.define_primitive("log", primitive_log);
    interp.define_primitive("expt", primitive_expt);
    interp.define_primitive("sqt", primitive_sqt);
    interp.define_primitive("sin", primitive_sin);
    interp.define_primitive("cos", primitive_cos);
    interp.define_primitive("tab", primitive_tan);
    interp.define_primitive("atan", primitive_atan);
}
