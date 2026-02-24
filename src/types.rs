use std::{cmp::Ordering, convert::TryFrom, fmt};

use crate::{interp::Scheme, markset::MarkSet};

pub type GcId = usize;

#[derive(Debug, PartialEq)]
pub enum SchemeError {
    EvalError(String),
    TypeError(String),
    UnboundVariable(String),
    SyntaxError(String),
    ImplementationError(String),
    ArgCountError(String),
    OverflowError(String),
    FileNotFound(String),
    UserError(String),
    IndexOutOfBounds(String),
    IOError(String),
    OutOfMemoryError(String),
    // Other error types can be added here
}

impl SchemeError {
    pub fn get_infos<'a>(&'a self) -> (&'static str, &'a str) {
        match self {
            SchemeError::EvalError(m) => ("Evaluation error", m),
            SchemeError::TypeError(m) => ("Type error", m),
            SchemeError::UnboundVariable(m) => ("Unbound variable", m),
            SchemeError::SyntaxError(m) => ("Syntax error", m),
            SchemeError::ImplementationError(m) => ("Internal error", m),
            SchemeError::ArgCountError(m) => ("Argument count Error", m),
            SchemeError::OverflowError(m) => ("Numeric overflow", m),
            SchemeError::FileNotFound(m) => ("File not found", m),
            SchemeError::UserError(m) => ("User error", m),
            SchemeError::IndexOutOfBounds(m) => ("Index out of bounds", m),
            SchemeError::IOError(m) => ("I/O error", m),
            SchemeError::OutOfMemoryError(m) => ("Out of memory error", m),
        }
    }
}

impl fmt::Display for SchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (label, message) = self.get_infos();
        write!(f, "[{}]: {}", label, message)
    }
}

impl From<std::io::Error> for SchemeError {
    fn from(err: std::io::Error) -> Self {
        SchemeError::IOError(err.to_string())
    }
}

// It is also good practice to implement the Error trait
impl std::error::Error for SchemeError {}

pub enum EvalResult {
    Done(Value),
    Continuation(Value, Value),
}

impl EvalResult {
    pub fn done(value: Value) -> Result<EvalResult, SchemeError> {
        Ok(EvalResult::Done(value))
    }
}
pub trait SchemeObject {
    fn eval(&self, interp: &Scheme, env: Value) -> Result<EvalResult, SchemeError>;
    fn is_false(&self) -> bool;
    fn display(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn write(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn mark(&self, interp: &Scheme, marks: &mut MarkSet);
}

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    Float(f64),
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => a == b,
            (Number::Int(a), Number::Float(b)) => (*a as f64) == *b,
            (Number::Float(a), Number::Int(b)) => *a == (*b as f64),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Integers print normally
            Number::Int(i) => write!(f, "{}", i),
            // Floats print as floating point numbers
            Number::Float(fl) => {
                // To ensure 5.0 doesn't just print as "5" in Scheme
                if fl.fract() == 0.0 {
                    write!(f, "{:.1}", fl)
                } else {
                    write!(f, "{}", fl)
                }
            }
        }
    }
}

impl std::ops::Add for Number {
    type Output = Number;

    fn add(self, other: Self) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a + b),
            (Number::Int(a), Number::Float(b)) => Number::Float(a as f64 + b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a + b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a + b),
        }
    }
}

impl std::ops::Mul for Number {
    type Output = Number;

    fn mul(self, other: Self) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a * b),
            (Number::Int(a), Number::Float(b)) => Number::Float(a as f64 * b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a * b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a * b),
        }
    }
}

impl std::ops::Neg for Number {
    type Output = Number;

    fn neg(self) -> Self::Output {
        match self {
            Number::Int(i) => Number::Int(-i),
            Number::Float(f) => Number::Float(-f),
        }
    }
}

impl std::ops::Sub for Number {
    type Output = Number;

    fn sub(self, other: Self) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a - b),
            (Number::Int(a), Number::Float(b)) => Number::Float(a as f64 - b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a - b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a - b),
        }
    }
}

impl std::ops::Div for Number {
    type Output = Number;

    fn div(self, other: Self) -> Self::Output {
        match (self, other) {
            // Strict promotion (simplest for now), even 4 / 2 becomes 2.0
            (Number::Int(a), Number::Int(b)) => Number::Float(a as f64 / b as f64),
            (Number::Int(a), Number::Float(b)) => Number::Float(a as f64 / b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a / b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a / b),
        }
    }
}

impl std::ops::Rem for Number {
    type Output = Number;

    fn rem(self, other: Self) -> Self::Output {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a % b),
            // For floats, Rust uses the same logic as f64.rem()
            (Number::Int(a), Number::Float(b)) => Number::Float(a as f64 % b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a % b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a % b),
        }
    }
}

impl Number {
    pub fn sqrt(&self) -> Number {
        match self {
            Number::Int(n) => Number::Float((*n as f64).sqrt()),
            Number::Float(f) => Number::Float(f.sqrt()),
        }
    }
}
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => a.partial_cmp(b),
            (Number::Float(a), Number::Float(b)) => a.partial_cmp(b),
            // Promotion: Convert Int to Float for comparison
            (Number::Int(a), Number::Float(b)) => (*a as f64).partial_cmp(b),
            (Number::Float(a), Number::Int(b)) => a.partial_cmp(&(*b as f64)),
        }
    }
}

impl TryFrom<Number> for i32 {
    type Error = String;

    fn try_from(value: Number) -> Result<i32, Self::Error> {
        match value {
            Number::Int(i) => {
                i32::try_from(i).map_err(|_| format!("Integer overflow {} to i32", value))
            }
            Number::Float(f) => {
                // Truncate the float and check range
                if f > i32::MAX as f64 || f < i32::MIN as f64 {
                    Err(format!("Float overflow {} to i32 range", value))
                } else {
                    Ok(f as i32)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Number(Number),
    Char(u8),
    Boolean(bool),
    Object(GcId),
    Nil,
    Unbound,
    Eof,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Number(_) => "Number",
            Self::Char(_) => "Char",
            Self::Boolean(_) => "Boolean",
            Self::Object(_) => "Object",
            Self::Nil => "Nil",
            Self::Unbound => "*unbound*",
            Self::Eof => "*EoF*",
        }
    }

    pub fn is_equal(&self, interp: &Scheme, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => {
                a == b || {
                    let heap = interp.heap.borrow();
                    let obja = heap.get(*a);
                    let objb = heap.get(*b);
                    obja.is_equal(interp, objb)
                }
            }
            (Value::Nil, Value::Nil) => true,
            (Value::Unbound, Value::Unbound) => false,
            _ => false,
        }
    }
}

impl SchemeObject for Value {
    fn eval(&self, interp: &Scheme, env: Value) -> Result<EvalResult, SchemeError> {
        match self {
            Value::Object(id) => id.eval(interp, env),
            _ => Ok(EvalResult::Done(*self)),
        }
    }

    fn is_false(&self) -> bool {
        match self {
            Value::Boolean(false) | Value::Nil => false,
            _ => true,
        }
    }

    fn write(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Object(id) => id.write(interp, f),
            Value::Number(n) => write!(f, "{}", n),
            Value::Char(ch) => {
                let ch = *ch as char;
                match ch {
                    '\x08' => write!(f, "#\\backspace"),
                    '\t' => write!(f, "#\\tab"),
                    ' ' => write!(f, "#\\space"),
                    '\n' => write!(f, "#\\newline"),
                    '\r' => write!(f, "#\\return"),
                    any => write!(f, "{}", any),
                }
            }
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::Nil => write!(f, "()"),
            Value::Unbound => write!(f, "*unbound*"),
            Value::Eof => write!(f, "*eof*"),
        }
    }

    fn display(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Object(id) => id.display(interp, f),
            Value::Number(n) => write!(f, "{}", n),
            Value::Char(ch) => {
                let ch = *ch as char;
                write!(f, "{}", ch)
            }
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::Nil => write!(f, "()"),
            Value::Unbound => write!(f, "*unbound*"),
            Value::Eof => write!(f, "*eof*"),
        }
    }

    fn mark(&self, interp: &Scheme, marks: &mut MarkSet) {
        match self {
            Value::Object(id) => id.mark(interp, marks),
            _ => (),
        }
    }
}
