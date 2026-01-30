use std::vec;

use scheme::types::Value;

use scheme::interp::Interp;


fn main() {
    let mut interp = Interp::new();

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

    let result = interp.eval(&list);
    match result {
        Ok(val) => interp.display(&val),
        Err(e) => eprintln!("Error: {:?}", e),
    }

}
