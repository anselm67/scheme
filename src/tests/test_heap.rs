use crate::heap::Heap;
use crate::interp::{Scheme, SchemeOptions};
use crate::types::SchemeError::UnboundVariable;
use crate::types::{Number, Value};

#[test]
fn test_intern_symbol() {
    let mut heap = Heap::new(128);

    let sym1 = heap.raw_intern_symbol("test").expect("test").1.id();
    let sym2 = heap.raw_intern_symbol("test").expect("test").1.id();

    assert_eq!(sym1, sym2, "Interned symbols should be the same");
}

#[tokio::test]
async fn test_eval_symbol() {
    let interp = Scheme::new(&SchemeOptions::new()).await;
    let mut heap = interp.heap.borrow_mut();
    // Creates an unbound symbol, and attempt to evaluate it.
    let symbol = heap
        .raw_intern_symbol("test-symbol")
        .expect("test-symbol")
        .1;
    drop(heap);

    let result = interp.eval(interp.env, symbol.value()).await;
    assert!(
        matches!(result, Err(UnboundVariable(_))),
        "Evaluated result should be an UnboundVariable error"
    );

    // Bind the symbol, check value.
    let value = Value::Number(Number::Int(32));
    interp.define_from_string("test-symbol", value);
    assert!(
        matches!(interp.eval(interp.env, symbol.value()).await, Ok(x) if x == value),
        "Evaluated symbol should return bound value"
    );
}

#[tokio::test]
async fn test_eval_string() {
    let interp = Scheme::new(&SchemeOptions::new()).await;
    let string = interp.alloc_string("Hello, World!").value();
    let Value::Object(string_id) = string else {
        panic!("Expected Value::Object");
    };
    let result = interp.eval(interp.env, string).await;
    assert!(
        matches!(result, Ok(Value::Object(id)) if id == string_id),
        "Evaluated string should return the same object ID"
    );
}

#[tokio::test]
async fn test_true_and_false_symbols() {
    let interp = Scheme::new(&SchemeOptions::new()).await;

    let true_sym = interp.intern_symbol("#t").1.value();
    let false_sym = interp.intern_symbol("#f").1.value();

    assert!(
        matches!(
            interp.eval(interp.env, true_sym).await,
            Ok(Value::Boolean(true))
        ),
        "#t should evaluate to Boolean(true)"
    );
    assert!(
        matches!(
            interp.eval(interp.env, false_sym).await,
            Ok(Value::Boolean(false))
        ),
        "#f should evaluate to Boolean(false)"
    );
}
