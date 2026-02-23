use crate::heap::Heap;
use crate::interp::{Scheme, SchemeOptions};
use crate::types::SchemeError::UnboundVariable;
use crate::types::{Number, Value};

#[test]
fn test_intern_symbol() {
    let mut heap = Heap::new(128);

    let sym1 = heap.raw_intern_symbol("test").expect("test").id();
    let sym2 = heap.raw_intern_symbol("test").expect("test").id();

    assert_eq!(sym1, sym2, "Interned symbols should be the same");
}

#[test]
fn test_eval_symbol() {
    let interp = Scheme::new(&SchemeOptions::new());
    let mut heap = interp.heap.borrow_mut();
    // Creates an unbound symbol, and attempt to evaluate it.
    let symbol = heap.raw_intern_symbol("test-symbol").expect("test-symbol");
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
    let interp = Scheme::new(&SchemeOptions::new());
    let string = interp.alloc_string("Hello, World!").value();
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
    let interp = Scheme::new(&SchemeOptions::new());

    let true_sym = interp.intern_symbol("#t").value();
    let false_sym = interp.intern_symbol("#f").value();

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
