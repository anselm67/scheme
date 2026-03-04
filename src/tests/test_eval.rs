use crate::{interp::SchemeOptions, types::Number};

#[tokio::test]
async fn test_eval_self_types() {
    use crate::interp::Scheme;
    use crate::types::Value;

    let interp = Scheme::new(&SchemeOptions::new()).await;

    let int_val = Value::Number(Number::Int(342));
    let float_val = Value::Number(Number::Float(3.14));
    let bool_val = Value::Boolean(true);
    let nil_val = Value::Nil;

    assert_eq!(interp.eval(interp.env, int_val).await.unwrap(), int_val);
    assert_eq!(interp.eval(interp.env, float_val).await.unwrap(), float_val);
    assert_eq!(interp.eval(interp.env, bool_val).await.unwrap(), bool_val);
    assert_eq!(interp.eval(interp.env, nil_val).await.unwrap(), nil_val);
}
