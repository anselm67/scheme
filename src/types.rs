use std::{cell::RefCell, rc::Rc, fmt, convert::TryFrom};

use crate::{env::Env, interp::Interp};

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
    // Other error types can be added here
}

pub trait SchemeObject {
    fn eval(&self, interp: &Interp, env: &Rc<RefCell<Env>>) -> Result<Value, SchemeError>;
    fn is_false(&self) -> bool;
    fn display(&self, interp: &Interp) -> String;
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

impl TryFrom<Number> for i32 {
    type Error = String;

    fn try_from(value: Number) -> Result<i32, Self::Error> {
        match value {
            Number::Int(i) => {
                i32::try_from(i).map_err(|_| format!("Integer overflow {} to i32", value))
            },
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
    Boolean(bool),
    Object(GcId),
    Nil
}

impl SchemeObject for Value {

    fn eval(&self, interp: &Interp, env: &Rc<RefCell<Env>>) -> Result<Value, SchemeError> {
        match self {
            Value::Object(id) => {
                id.eval(interp, env)
            },
            _ => Ok(*self),
        }
    }

    fn is_false(&self) -> bool {
        match self {
            Value::Boolean(false) | Value::Nil => false,
            _ => true,
        }
    }

    fn display(&self, interp: &Interp) -> String {
        match self {
            Value::Object(id) => id.display(interp),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
        }
    }

}

