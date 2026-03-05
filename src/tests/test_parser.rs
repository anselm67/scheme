use crate::{
    interp::{Scheme, SchemeOptions},
    parser::Parser,
};

#[tokio::test]
async fn test_parse_some_exprs() {
    let interp = Scheme::new(&SchemeOptions::new()).await;

    let inputs = vec![
        "(* 2 3)",
        "(1 2 3)",
        "((lambda (x) (+ x 1)) 2)",
        "'(1 2 . 3)",
    ];
    for text in inputs {
        let mut parser = Parser::from_string(text);
        let expr = parser.read(&interp).await;
        assert!(matches!(expr, Ok(_)));
    }
}

#[tokio::test]
async fn test_parse_fails() {
    let interp = Scheme::new(&SchemeOptions::new()).await;

    let inputs = vec!["(* 2 3", "(define! x \\#a)"];
    for text in inputs {
        let mut parser = Parser::from_string(text);
        let expr = parser.read(&interp).await;
        assert!(matches!(expr, Err(_)));
    }
}
