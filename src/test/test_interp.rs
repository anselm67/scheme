use crate::{interp::Interp, parser::Parser, types::{Number, Value}};


fn eval_expr(interp: &Interp, expr: Value) {
    interp.display(expr);
    let result = interp.eval(expr);
    match result {
        Ok(val) => println!("{}", interp.display(val)),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn check_exprs(interp: &Interp, inputs: &Vec<(&str, Value)>) {
    for (text, expected) in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        match expr {
            Ok(expr) => {
                match interp.eval(expr) {
                    Ok(value) => assert_eq!(value, *expected),
                    Err(e) => panic!("Eval {} failed with error: {:?}", text, e)
                }
            },
            Err(e) => panic!("Parse {} failed, error: {:?}.", text, e)
        }
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
        Value::Number(Number::Int(42)),
        Value::Number(Number::Int(0)),
    ]);

    let cond_expr_false = heap.alloc_list(vec![
        cond,
        fls,
        Value::Number(Number::Int(42)),
        Value::Number(Number::Int(0)),
    ]);
    drop(heap);

    eval_expr(&interp, cond_expr_true);
    eval_expr(&interp, cond_expr_false);
}  

#[test]
fn test_nested_expr() {
    let interp = Interp::new();
    
    let add = interp.lookup("+");
    let mul = interp.lookup("*");
    let mut heap = interp.heap.borrow_mut();

    let expr= heap.alloc_list(vec![
        mul,
        Value::Number(Number::Int(2)),
        Value::Number(Number::Int(3)),
    ]);

    let list: Value = heap.alloc_list(vec![
        add,
        expr,
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
    ]);
    drop(heap);

    eval_expr(&interp, list);
}


#[test]
fn test_setbang_special_form() {
    let interp = Interp::new();
    
    let define = interp.lookup("define");
    let x = interp.lookup("x");

    let mut heap = interp.heap.borrow_mut();

    let expr= heap.alloc_list(vec![
        define,
        x,
        Value::Number(Number::Int(1))
    ]);
    drop(heap);
    
    eval_expr(&interp, expr);
    eval_expr(&interp, x);
}

#[test]
fn test_read_eval_number() {
    let inputs = vec![
        ("(* 3 2)", Value::Number(Number::Int(6))),
        ("(- 1)",  Value::Number(Number::Int(-1))),
        ("(- 2 1)",  Value::Number(Number::Int(1))),
        ("(/ 2)",  Value::Number(Number::Float(0.5))),
        ("(/ 4 2)",  Value::Number(Number::Float(2.0))),
        ("(% 10 3)",  Value::Number(Number::Int(1))),
        ("(= 10. 10.0)",  Value::Boolean(true)),
        ("(> 10 3)",  Value::Boolean(true)),
        ("(>= 10 10)",  Value::Boolean(true)),
        ("(< 10 3)",  Value::Boolean(false)),
        ("(<= 3 3)",  Value::Boolean(true)),
        ("(number? 1)",  Value::Boolean(true)),
        ("(number? \"x\")",  Value::Boolean(false)),
        ("(integer? 1)",  Value::Boolean(true)),
        ("(integer? 1.0)",  Value::Boolean(false)),
        ("(float? 1.0)",  Value::Boolean(true)),
        ("(float? 1)",  Value::Boolean(false)),
        ("(max 4 2.0 1)",  Value::Number(Number::Int(4))),
        ("(min 4 2.0 7)",  Value::Number(Number::Float(2.0))),
    ];
    let interp = Interp::new();
    check_exprs(&interp, &inputs);
}


#[test]
fn test_read_eval_closure() {
    let inputs = vec![
        ("((lambda (x) (+ x 1)) 2)", Value::Number(Number::Int(3))),
        ("((lambda (x y) (+ x y)) 1 2)", Value::Number(Number::Int(3))),
    ];
    let interp = Interp::new();
    check_exprs(&interp, &inputs);
}


#[test]
fn test_read_eval_list() {
    let inputs = vec![
        ("(list? '(1 2))", Value::Boolean(true)),
        ("(list? \"hello\")", Value::Boolean(false)),
        ("(null? '(1 2))')", Value::Boolean(false)),
        ("(null? ())", Value::Boolean(true)),
        ("(car '(1 2))", Value::Number(Number::Int(1))),
        ("(car (cdr '(1 2)))", Value::Number(Number::Int(2))),
    ];
    let interp = Interp::new();
    check_exprs(&interp, &inputs);
}


#[test]
fn test_read_eval_char() {
    let inputs = vec![
        ("(char? #\\A)", Value::Boolean(true)),
        ("(char? 10)", Value::Boolean(false)),
        ("(char->integer #\\A)", Value::Number(Number::Int(65))),
        ("(char->integer #\\A)", Value::Number(Number::Int(65))),
        ("(integer->char 65)", Value::Char(65)),
        ("(char=? #\\a #\\a)", Value::Boolean(true)),
        ("(char=? #\\b #\\a)", Value::Boolean(false)),
        ("(char>? #\\a #\\b)", Value::Boolean(false)),
        ("(char<? #\\a #\\b)", Value::Boolean(true)),
        ("(char>=? #\\a #\\a)", Value::Boolean(true)),
        ("(char<=? #\\a #\\a)", Value::Boolean(true)),
        ("(char-ci=? #\\B #\\a)", Value::Boolean(false)),
        ("(char-ci>? #\\A #\\b)", Value::Boolean(false)),
        ("(char-ci<? #\\A #\\b)", Value::Boolean(true)),
        ("(char-ci>=? #\\A #\\a)", Value::Boolean(true)),
        ("(char-ci<=? #\\A #\\a)", Value::Boolean(true)),
    ];
    let interp = Interp::new();
    check_exprs(&interp, &inputs);
}

