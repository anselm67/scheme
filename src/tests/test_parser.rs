use crate::{
    interp::{Scheme, SchemeOptions},
    parser::Parser,
};

#[test]
fn test_parse_some_exprs() {
    let interp = Scheme::new(&SchemeOptions::new());

    let inputs = vec![
        "(* 2 3)",
        "(1 2 3)",
        "((lambda (x) (+ x 1)) 2)",
        "'(1 2 . 3)",
    ];
    for text in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        assert!(matches!(expr, Ok(_)));
    }
}

#[test]
fn test_parse_fails() {
    let interp = Scheme::new(&SchemeOptions::new());

    let inputs = vec!["(* 2 3", "(define! x \\#a)"];
    for text in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        assert!(matches!(expr, Err(_)));
    }
}
