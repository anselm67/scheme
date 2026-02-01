use std::vec;

use scheme::types::Value;

use scheme::interp::{Interp};

fn eval_expr(interp: &Interp, expr: &Value) {
    interp.display(expr);
    let result = interp.eval(expr);
    match result {
        Ok(val) => interp.display(&val),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn test_cond(interp: &Interp) {
    let cond = interp.lookup("if");
    let tru = interp.lookup("#t");
    let fls = interp.lookup("#f");

    let mut heap = interp.heap.borrow_mut();

    let cond_expr_true = heap.alloc_list(vec![
        cond,
        tru,
        Value::Integer(42),
        Value::Integer(0),
    ]);

    let cond_expr_false = heap.alloc_list(vec![
        cond,
        fls,
        Value::Integer(42),
        Value::Integer(0),
    ]);
    drop(heap);

    eval_expr(interp, &cond_expr_true);
    eval_expr(interp, &cond_expr_false);
}   

fn test_nested_expr(interp: &Interp) {
    let add = interp.lookup("+");
    let mul = interp.lookup("*");
    let mut heap = interp.heap.borrow_mut();

    let expr= heap.alloc_list(vec![
        mul,
        Value::Integer(2),
        Value::Integer(3),
    ]);

    let list: Value = heap.alloc_list(vec![
        add,
        expr,
        Value::Integer(1),
        Value::Integer(2),
    ]);
    drop(heap);
    eval_expr(interp, &list);
}

fn test_lambda() {
    let interp = Interp::new();

    let x = interp.lookup("x");
    let add = interp.lookup("+");
    let lambda = interp.lookup("lambda");

    let mut heap = interp.heap.borrow_mut();

    let params = heap.alloc_list(vec![x]);
    let body = heap.alloc_list(vec![
        add,
        x,
        Value::Integer(1),
    ]);
    let lambda = heap.alloc_list(vec![
        lambda,
        params,
        body,
    ]);
    let call = heap.alloc_list(vec![
        lambda,
        Value::Integer(41),
    ]);
    drop(heap);

    eval_expr(&interp, &call);
}


fn main() {
    let interp = Interp::new();

    test_cond(&interp);
    test_nested_expr(&interp);
    test_lambda()
}
