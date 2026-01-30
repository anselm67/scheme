use crate::interp::Interp;

pub type GcId = usize;

#[derive(Debug)]
pub enum SchemeError {
    EvalError(String),
    TypeError(String),
    // Other error types can be added here
}

pub trait SchemeObject {
    fn eval(&self, interp: &Interp) -> Result<Value, SchemeError>;
    fn display(&self, interp: &Interp) -> String;
}

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Object(GcId),
    Nil
}

impl SchemeObject for Value {

    fn eval(&self, interp: &Interp) -> Result<Value, SchemeError> {
        match self {
            Value::Integer(_) | Value::Float(_) | Value::Boolean(_) => Ok(*self),
            Value::Object(id) => {
                id.eval(interp)
            },
            _ => Ok(*self),
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

