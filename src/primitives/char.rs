use std::{cell::RefCell, rc::Rc};

use crate::{
    env::Env, 
    interp::{ Interp }, 
    types::{Value, Number, EvalResult, SchemeError},
};

fn primitive_char_p(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_char(args[0]).is_some()))
}

fn primitive_char_alphabetic_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Boolean((*ch as char).is_alphabetic()))
}

fn primitive_char_numeric_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Boolean((*ch as char).is_digit(10)))
}

fn primitive_char_whitespace_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Boolean(*ch == 9 || *ch == 10 || *ch == 32))
}

fn primitive_char_upper_case_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Boolean((*ch as char).is_uppercase()))
}

fn primitive_char_lower_case_p(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Boolean((*ch as char).is_lowercase()))
}

fn primitive_char_to_integer(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Number(Number::Int(*ch as i64)))
}

fn primitive_integer_to_char(interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    check_arity!(args, 1);
    let byte = interp.to_integer(args[0])?;
    EvalResult::done(Value::Char(byte as u8))
}

fn primitive_char_upcase(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Char((*ch as char).to_ascii_uppercase() as u8))
}

fn primitive_char_downcase(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 1, ch: Char);
    EvalResult::done(Value::Char((*ch as char).to_ascii_lowercase() as u8))
}

fn primitive_char_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1 == ch2))
}

fn primitive_char_lt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1 < ch2))
}

fn primitive_char_lte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1 <= ch2))
}

fn primitive_char_gt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1 > ch2))
}

fn primitive_char_gte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1 >= ch2))
}

fn primitive_char_ci_eq(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1.to_ascii_lowercase() == ch2.to_ascii_lowercase()))
}

fn primitive_char_ci_lt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1.to_ascii_lowercase() < ch2.to_ascii_lowercase()))
}

fn primitive_char_ci_lte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1.to_ascii_lowercase() <= ch2.to_ascii_lowercase()))
}

fn primitive_char_ci_gt(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1.to_ascii_lowercase() > ch2.to_ascii_lowercase()))
}

fn primitive_char_ci_gte(_interp: &Interp, _env: Rc<RefCell<Env>>, args: &[Value]) 
    -> Result<EvalResult, SchemeError> 
{
    extract_args!(args, 2, ch1: Char, ch2: Char);
    EvalResult::done(Value::Boolean(ch1.to_ascii_lowercase() >= ch2.to_ascii_lowercase()))
}

pub fn register(interp: &Interp) {
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