use crate::heap::Heap;
use crate::interp::Interp;
use crate::types::SchemeError::UnboundVariable;
use crate::types::Value;

#[test]
fn test_intern_symbol() {

    let mut heap = Heap::new();

    let sym1 = heap.intern_symbol("test");
    let sym2 = heap.intern_symbol("test");

    assert_eq!(sym1, sym2, "Interned symbols should be the same");
}


#[test]
fn test_eval_symbol() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();
    // Creates an unbound symbol, and attempt to evaluate it.
    let symbol = heap.intern_symbol("test-symbol");
    drop(heap);

    let result = interp.eval(symbol);
    assert!(matches!(result, Err(UnboundVariable(_))), "Evaluated result should be an UnboundVariable error");

    // Bind the symbol, check valye.
    interp.define("test-symbol", Value::Integer(42));
    assert!(matches!(interp.eval(symbol), Ok(Value::Integer(42))), "Evaluated symbol should return bound value");
}

#[test]
fn test_eval_string() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();
    let string = heap.alloc_string("Hello, World!");
    drop(heap);
    let Value::Object(string_id) = string else {
        panic!("Expected Value::Object");
    };
    let result = interp.eval(string);
    assert!(matches!(result, Ok(Value::Object(id)) if id == string_id), "Evaluated string should return the same object ID");
}

#[test]
fn test_true_and_false_symbols() {
    let interp = Interp::new();
    let mut heap = interp.heap.borrow_mut();

    let true_sym = heap.intern_symbol("#t");
    let false_sym = heap.intern_symbol("#f");
    drop(heap);
    
    assert!(matches!(interp.eval(true_sym), Ok(Value::Boolean(true))), "#t should evaluate to Boolean(true)");
    assert!(matches!(interp.eval(false_sym), Ok(Value::Boolean(false))), "#f should evaluate to Boolean(false)");  
}