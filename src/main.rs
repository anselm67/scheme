use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use scheme::parser::Parser;
use scheme::types::Value;

use scheme::interp::Interp;

fn eval_expr(interp: &Interp, expr: Value) {
    let expansion = interp.expand(expr);
    expansion
        .and_then(|expanded| {
            let env = interp.env.clone();
            interp.eval(env, expanded.value())
        })
        .map(|value| println!(" = {}", interp.display(value)))
        .unwrap_or_else(|e| eprintln!("{}", e));
}

const HISTORY_FILENAME: &str = ".scheme.history";

fn repl(interp: &Interp) {
    let mut rl = DefaultEditor::new().expect("Failed to init REPL.");

    if rl.load_history(HISTORY_FILENAME).is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                let mut parser = Parser::new(line.as_bytes());
                let expr = parser.read(interp);
                match expr {
                    Ok(expr) => eval_expr(interp, expr.value()),
                    Err(e) => eprintln!("Error: {:?}", e),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history(HISTORY_FILENAME)
        .expect(format!("Failed to save history to {}.", HISTORY_FILENAME).as_str());
}

fn main() {
    let interp = Interp::new();

    repl(&interp);
}
