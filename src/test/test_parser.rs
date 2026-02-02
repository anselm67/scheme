use crate::{interp::Interp, parser::Parser};


#[test]
fn test_parse_some_exprs() {
    let interp = Interp::new();

    let inputs = vec![
        "(* 2 3)",
        "(1 2 3)",
        "((lambda (x) (+ x 1)) 2)"
    ];
    for text in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        assert!(matches!(expr, Ok(_)));
    }
}

#[test]
fn test_parse_fails() {
    let interp = Interp::new();

    let inputs = vec![
        "(*, 2 3)",
    ];
    for text in inputs {
        let mut parser = Parser::new(text.as_bytes());
        let expr = parser.read(&interp);
        assert!(matches!(expr, Err(_)));
    }
}