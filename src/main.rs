use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use scheme::parser::Parser;
use scheme::types::Value;

use scheme::interp::{Scheme, SchemeOptions};

fn eval_expr(interp: &Scheme, expr: Value) {
    let expansion = interp.expand(expr);
    expansion
        .and_then(|expanded| {
            if interp.debug_macro {
                println!("expanded => {}", interp.display(expanded.value()));
            }
            interp.eval(interp.env, expanded.value())
        })
        .map(|value| {
            interp.flush_stdout();
            println!(" = {}", interp.display(value))
        })
        .unwrap_or_else(|e| eprintln!("{}", e));
}

const HISTORY_FILENAME: &str = ".scheme.history";

fn repl(interp: &Scheme) {
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

#[derive(clap::Parser, Debug)]
#[command(author, version, about="A rusty Scheme interpreter.", long_about=None)]
struct Arg {
    /// List of files to load upon startup.
    files: Vec<String>,

    /// Do not load the Scheme code that implements full R4RS compliance.
    #[arg(long = "no-init", default_value_t = true, action=clap::ArgAction::SetFalse)]
    init: bool,

    /// Debug macro by printing expressions before / after expansion.
    #[arg(long)]
    debug_macro: bool,

    /// Initial heap size in number of objects.
    #[arg(short = 's', long, default_value_t = 8192)]
    heap_size: usize,
}

fn main() {
    let arg = <Arg as clap::Parser>::parse();
    let options = SchemeOptions::new()
        .set_init_scheme(arg.init)
        .set_debug_macro(arg.debug_macro)
        .set_heap_size(arg.heap_size);

    let interp = Scheme::new(&options);
    for file in &arg.files {
        println!("Loading {}", file);
        match interp.load(file) {
            Err(e) => {
                panic!("Failed to load {}: {}", file.to_string(), e);
            }
            _ => {}
        }
    }
    repl(&interp);
}
