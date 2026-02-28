use crate::{
    interp::Scheme,
    types::{EvalResult, SchemeError, Value},
};

fn primitive_char_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(interp.is_char(args[0]).is_some())
}

fn primitive_char_alphabetic_p(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::bool((*ch as char).is_alphabetic())
}

fn primitive_char_numeric_p(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::bool((*ch as char).is_digit(10))
}

fn primitive_char_whitespace_p(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::bool(*ch == 9 || *ch == 10 || *ch == 32)
}

fn primitive_char_upper_case_p(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::bool((*ch as char).is_uppercase())
}

fn primitive_char_lower_case_p(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::bool((*ch as char).is_lowercase())
}

fn primitive_char_to_integer(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::int(*ch as i64)
}

fn primitive_integer_to_char(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let byte = interp.to_integer(args[0])?;
    EvalResult::char((byte as u8) as char)
}

fn primitive_char_upcase(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::char((*ch as char).to_ascii_uppercase())
}

fn primitive_char_downcase(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, ch: Char);
    EvalResult::char((*ch as char).to_ascii_lowercase())
}

fn primitive_char_eq(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1 == ch2)
}

fn primitive_char_lt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1 < ch2)
}

fn primitive_char_lte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1 <= ch2)
}

fn primitive_char_gt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1 > ch2)
}

fn primitive_char_gte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1 >= ch2)
}

fn primitive_char_ci_eq(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1.to_ascii_lowercase() == ch2.to_ascii_lowercase())
}

fn primitive_char_ci_lt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1.to_ascii_lowercase() < ch2.to_ascii_lowercase())
}

fn primitive_char_ci_lte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1.to_ascii_lowercase() <= ch2.to_ascii_lowercase())
}

fn primitive_char_ci_gt(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1.to_ascii_lowercase() > ch2.to_ascii_lowercase())
}

fn primitive_char_ci_gte(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::bool(ch1.to_ascii_lowercase() >= ch2.to_ascii_lowercase())
}

pub fn register(interp: &Scheme) {
    interp.define_primitive("char?", primitive_char_p);
    interp.define_primitive("char-alphabetic?", primitive_char_alphabetic_p);
    interp.define_primitive("char-numeric?", primitive_char_numeric_p);
    interp.define_primitive("char-whitespace?", primitive_char_whitespace_p);
    interp.define_primitive("char-upper-case?", primitive_char_upper_case_p);
    interp.define_primitive("char-lower-case?", primitive_char_lower_case_p);
    interp.define_primitive("char->integer", primitive_char_to_integer);
    interp.define_primitive("integer->char", primitive_integer_to_char);
    interp.define_primitive("char-upcase", primitive_char_upcase);
    interp.define_primitive("char-downcase", primitive_char_downcase);
    interp.define_primitive("char=?", primitive_char_eq);
    interp.define_primitive("char<?", primitive_char_lt);
    interp.define_primitive("char<=?", primitive_char_lte);
    interp.define_primitive("char>?", primitive_char_gt);
    interp.define_primitive("char>=?", primitive_char_gte);
    interp.define_primitive("char-ci=?", primitive_char_ci_eq);
    interp.define_primitive("char-ci<?", primitive_char_ci_lt);
    interp.define_primitive("char-ci<=?", primitive_char_ci_lte);
    interp.define_primitive("char-ci>?", primitive_char_ci_gt);
    interp.define_primitive("char-ci>=?", primitive_char_ci_gte);
}
