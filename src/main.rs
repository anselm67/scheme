use std::vec;

use scheme::types::Value;

use scheme::interp::Interp;


fn main() {
    let interp = Interp::new();

    let cond = interp.lookup("if");
    let tru = interp.lookup("#f");

    let mut heap = interp.heap.borrow_mut();
    let cond_expr = heap.alloc_list(vec![
        cond,
        tru,
        Value::Integer(42),
        Value::Integer(0),
    ]);
    drop(heap);
    interp.display(&cond_expr);

    /*
    let add = interp.lookup("+");
    let mul = interp.lookup("*");

    let expr= interp.heap.alloc_list(vec![
        mul,
        Value::Integer(2),
        Value::Integer(3),
    ]);

    let list: Value = interp.heap.alloc_list(vec![
        add,
        expr,
        Value::Integer(1),
        Value::Integer(2),
    ]);

    interp.display(&list);
    */

    let result = interp.eval(&cond_expr);
    match result {
        Ok(val) => interp.display(&val),
        Err(e) => eprintln!("Error: {:?}", e),
    }

}
