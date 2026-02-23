use crate::{interp::SchemeOptions, types::Number};

#[test]
fn test_eval_self_types() {
    use crate::interp::Scheme;
    use crate::types::Value;

    let interp = Scheme::new(&SchemeOptions::new());

    let int_val = Value::Number(Number::Int(342));
    let float_val = Value::Number(Number::Float(3.14));
    let bool_val = Value::Boolean(true);
    let nil_val = Value::Nil;

    assert_eq!(interp.eval(interp.env.clone(), int_val).unwrap(), int_val);
    assert_eq!(
        interp.eval(interp.env.clone(), float_val).unwrap(),
        float_val
    );
    assert_eq!(interp.eval(interp.env.clone(), bool_val).unwrap(), bool_val);
    assert_eq!(interp.eval(interp.env.clone(), nil_val).unwrap(), nil_val);
}
