use std::{cell::RefCell, rc::Rc};

use crate::{env::Env, interp::Interp};

pub type GcId = usize;

#[derive(Debug, PartialEq)]
pub enum SchemeError {
    EvalError(String),
    TypeError(String),
    UnboundVariable(String),
    SyntaxError(String),
    ImplementationError(String),
    // Other error types can be added here
}

pub trait SchemeObject {
    fn eval(&self, interp: &Interp, env: &Rc<RefCell<Env>>) -> Result<Value, SchemeError>;
    fn is_false(&self) -> bool;
    fn display(&self, interp: &Interp) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
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
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
        }
    }

}

