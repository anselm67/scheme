use crate::heap::Heap;
use crate::interp::Interp;
use crate::types::SchemeError::UnboundVariable;
use crate::types::{Number, Value};

#[test]
fn test_intern_symbol() {
    let mut heap = Heap::new(128);

    let sym1 = heap.intern_symbol("test").id();
    let sym2 = heap.intern_symbol("test").id();

    assert_eq!(sym1, sym2, "Interned symbols should be the same");
}

#[test]
fn test_eval_symbol() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();
    // Creates an unbound symbol, and attempt to evaluate it.
    let symbol = heap.intern_symbol("test-symbol");
    drop(heap);

    let result = interp.eval(interp.env.clone(), symbol.value());
    assert!(
        matches!(result, Err(UnboundVariable(_))),
        "Evaluated result should be an UnboundVariable error"
    );

    // Bind the symbol, check value.
    let value = Value::Number(Number::Int(32));
    interp.define("test-symbol", value);
    assert!(
        matches!(interp.eval(interp.env.clone(), symbol.value()), Ok(x) if x == value),
        "Evaluated symbol should return bound value"
    );
}

#[test]
fn test_eval_string() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();
    let string = heap.alloc_string("Hello, World!").value();
    drop(heap);
    let Value::Object(string_id) = string else {
        panic!("Expected Value::Object");
    };
    let result = interp.eval(interp.env.clone(), string);
    assert!(
        matches!(result, Ok(Value::Object(id)) if id == string_id),
        "Evaluated string should return the same object ID"
    );
}

#[test]
fn test_true_and_false_symbols() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();

    let true_sym = heap.intern_symbol("#t").value();
    let false_sym = heap.intern_symbol("#f").value();
    drop(heap);

    assert!(
        matches!(
            interp.eval(interp.env.clone(), true_sym),
            Ok(Value::Boolean(true))
        ),
        "#t should evaluate to Boolean(true)"
    );
    assert!(
        matches!(
            interp.eval(interp.env.clone(), false_sym),
            Ok(Value::Boolean(false))
        ),
        "#f should evaluate to Boolean(false)"
    );
}
