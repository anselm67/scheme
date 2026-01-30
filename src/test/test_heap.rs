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
    let mut interp = Interp::new();

    // Creates an unbound symbol, and attempt to evaluate it.
    let symbol = interp.heap.intern_symbol("test-symbol");
    let result = interp.eval(&symbol);
    assert!(matches!(result, Err(UnboundVariable(_))), "Evaluated result should be an UnboundVariable error");

    // Bind the symbol, check valye.
    interp.define("test-symbol", Value::Integer(42));
    assert!(matches!(interp.eval(&symbol), Ok(Value::Integer(42))), "Evaluated symbol should return bound value");
}