use crate::{interp::Interp, types::Value};


fn eval_expr(interp: &Interp, expr: &Value) {
    interp.display(expr);
    let result = interp.eval(expr);
    match result {
        Ok(val) => interp.display(&val),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

#[test]
fn test_cond() {
    let interp = Interp::new();
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

    eval_expr(&interp, &cond_expr_true);
    eval_expr(&interp, &cond_expr_false);
}  

#[test]
fn test_nested_expr() {
    let interp = Interp::new();
    
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
    
    eval_expr(&interp, &list);
}
