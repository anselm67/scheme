#[test]
fn test_eval_self_types() {
    use crate::interp::Interp;
    use crate::types::Value;

    let interp = Interp::new();

    let int_val = Value::Integer(42);
    let float_val = Value::Float(3.14);
    let bool_val = Value::Boolean(true);
    let nil_val = Value::Nil;

    assert_eq!(interp.eval(int_val).unwrap(), int_val);
    assert_eq!(interp.eval(float_val).unwrap(), float_val);
    assert_eq!(interp.eval(bool_val).unwrap(), bool_val);
    assert_eq!(interp.eval(nil_val).unwrap(), nil_val);
}
