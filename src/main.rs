use std::io::Write;
use std::{io, process};

use scheme::parser::Parser;
use scheme::types::Value;

use scheme::interp::{Interp};

fn eval_expr(interp: &Interp, expr: Value) {
    let result = interp.eval(expr);
    match result {
        Ok(val) => {
            print!(" = ");
            interp.display(val)
        },
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn repl(interp: &Interp) {
    let input = io::stdin();
    let mut parser = Parser::new(input);
    
    loop {
        print!("? ");
        io::stdout().flush().unwrap();
        let expr = parser.read(interp);
        match expr {
            Ok(Value::Nil) => process::exit(0),
            Ok(expr) => eval_expr(interp, expr),
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }
}

fn main() {
    let interp = Interp::new();

    repl(&interp);
}
