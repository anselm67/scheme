use crate::{
    interp::{Scheme, SchemeOptions},
    parser::Parser,
    types::{Number, SchemeError, Value},
};

fn eval_expr(interp: &Scheme, expr: Value) {
    interp.display(expr);
    let result = interp.eval(interp.env.clone(), expr);
    match result {
        Ok(val) => println!("{}", interp.display(val)),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn check_exprs(interp: &Scheme, inputs: &Vec<(&str, Value)>) {
    for (text, expected) in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        match expr {
            Ok(expr) => match interp.eval(interp.env.clone(), expr.value()) {
                Ok(value) => assert_eq!(value, *expected),
                Err(e) => panic!("Eval {} failed with error: {:?}", text, e),
            },
            Err(e) => panic!("Parse {} failed, error: {:?}.", text, e),
        }
    }
}

fn check_errors(interp: &Scheme, inputs: &Vec<(&str, SchemeError)>) {
    for (text, expected) in inputs {
        let mut parser = Parser::new(text.as_bytes());
        if let Ok(expr) = parser.read(&interp) {
            match interp.eval(interp.env.clone(), expr.value()) {
                Ok(_) => panic!("Failure was expected, but success happened!"),
                Err(e) => assert_eq!(e, *expected),
            }
        } else {
            panic!("check_errors: couldn't parse {}", text);
        }
    }
}

#[test]
fn test_cond() {
    let interp = Scheme::new(&SchemeOptions::new());
    let cond = interp.lookup("if");
    let tru = interp.lookup("#t");
    let fls = interp.lookup("#f");

    let cond_expr_true = interp.alloc_list(&[
        cond.value(),
        tru.value(),
        Value::Number(Number::Int(42)),
        Value::Number(Number::Int(0)),
    ]);

    let cond_expr_false = interp.alloc_list(&[
        cond.value(),
        fls.value(),
        Value::Number(Number::Int(42)),
        Value::Number(Number::Int(0)),
    ]);

    eval_expr(&interp, cond_expr_true.value());
    eval_expr(&interp, cond_expr_false.value());
}

#[test]
fn test_nested_expr() {
    let interp = Scheme::new(&SchemeOptions::new());

    let add = interp.lookup("+");
    let mul = interp.lookup("*");

    let expr = interp.alloc_list(&[
        mul.value(),
        Value::Number(Number::Int(2)),
        Value::Number(Number::Int(3)),
    ]);

    let list = interp.alloc_list(&[
        add.value(),
        expr.value(),
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
    ]);

    eval_expr(&interp, list.value());
}

#[test]
fn test_setbang_special_form() {
    let interp = Scheme::new(&SchemeOptions::new());

    let define = interp.lookup("define").value();
    let x = interp.lookup("x");

    let expr = interp.alloc_list(&[define, x.value(), Value::Number(Number::Int(1))]);

    eval_expr(&interp, expr.value());
    eval_expr(&interp, x.value());
}

#[test]
fn test_read_eval_number() {
    let inputs = vec![
        ("(* 3 2)", Value::Number(Number::Int(6))),
        ("(- 1)", Value::Number(Number::Int(-1))),
        ("(- 2 1)", Value::Number(Number::Int(1))),
        ("(/ 2)", Value::Number(Number::Float(0.5))),
        ("(/ 4 2)", Value::Number(Number::Float(2.0))),
        ("(% 10 3)", Value::Number(Number::Int(1))),
        ("(= 10. 10.0)", Value::Boolean(true)),
        ("(> 10 3)", Value::Boolean(true)),
        ("(>= 10 10)", Value::Boolean(true)),
        ("(< 10 3)", Value::Boolean(false)),
        ("(<= 3 3)", Value::Boolean(true)),
        ("(number? 1)", Value::Boolean(true)),
        ("(number? \"x\")", Value::Boolean(false)),
        ("(integer? 1)", Value::Boolean(true)),
        ("(integer? 1.0)", Value::Boolean(false)),
        ("(float? 1.0)", Value::Boolean(true)),
        ("(float? 1)", Value::Boolean(false)),
        ("(max 4 2.0 1)", Value::Number(Number::Int(4))),
        ("(min 4 2.0 7)", Value::Number(Number::Float(2.0))),
    ];
    let interp = Scheme::new(&SchemeOptions::new());
    check_exprs(&interp, &inputs);
}

#[test]
fn test_read_eval_closure() {
    let inputs = vec![
        (
            "((lambda (x . y) (length y)) 1 2 3)",
            Value::Number(Number::Int(2)),
        ),
        ("((lambda (x) (+ x 1)) 2)", Value::Number(Number::Int(3))),
        ("((lambda (x) (+ x 1)) 2)", Value::Number(Number::Int(3))),
        (
            "((lambda (x y) (+ x y)) 1 2)",
            Value::Number(Number::Int(3)),
        ),
    ];
    let interp = Scheme::new(&SchemeOptions::new());
    check_exprs(&interp, &inputs);
}

#[test]
fn test_read_eval_list() {
    let inputs = vec![
        ("(list? '(1 2))", Value::Boolean(true)),
        ("(append)", Value::Nil),
        ("(length '(1 2))", Value::Number(Number::Int(2))),
        ("(length ())", Value::Number(Number::Int(0))),
        ("(list? \"hello\")", Value::Boolean(false)),
        ("(null? '(1 2))')", Value::Boolean(false)),
        ("(null? ())", Value::Boolean(true)),
        ("(car '(1 2))", Value::Number(Number::Int(1))),
        ("(car (cdr '(1 2)))", Value::Number(Number::Int(2))),
        ("(car '(1 . 2))", Value::Number(Number::Int(1))),
        ("(cdr '(1 . 2))", Value::Number(Number::Int(2))),
    ];
    let interp = Scheme::new(&SchemeOptions::new());
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
    let interp = Scheme::new(&SchemeOptions::new());
    check_exprs(&interp, &inputs);
}

#[test]
fn test_read_eval_functional() {
    let inputs = vec![(
        "(eval (append (list (list (quote lambda) (quote (x y)) (quote (+ x y))) 1 2)))",
        Value::Number(Number::Int(3)),
    )];
    let interp = Scheme::new(&SchemeOptions::new());
    check_exprs(&interp, &inputs);
}

#[test]
fn test_equality() {
    let inputs = vec![
        ("(eq? 1 1)", Value::Boolean(true)),
        ("(eq? 1 2)", Value::Boolean(false)),
        ("(eq? (list 1) (list 1))", Value::Boolean(false)),
        ("(equal? \"a\" \"a\")", Value::Boolean(true)),
        ("(eq? \"a\" \"b\")", Value::Boolean(false)),
        ("(equal? (list 1) (list 1))", Value::Boolean(true)),
        ("(equal? (cons 1 2) (cons 1 2))", Value::Boolean(true)),
    ];
    let interp = Scheme::new(&SchemeOptions::new());
    check_exprs(&interp, &inputs);
}

#[test]
fn test_user_error() {
    let inputs = vec![("(error \"a\")", SchemeError::UserError("a".to_string()))];
    let interp = Scheme::new(&SchemeOptions::new());
    check_errors(&interp, &inputs);
}
