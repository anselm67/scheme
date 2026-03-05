use clap::ArgAction;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use scheme::parser::Parser;
use scheme::types::Value;

use scheme::interp::{Scheme, SchemeOptions};

async fn eval_expr(interp: &Scheme, expr: Value) {
    let expansion = interp.expand(expr).await;
    match expansion {
        Ok(expanded) => {
            if interp.debug_macro {
                println!("expanded => {}", interp.display(expanded.value()));
            }
            let result = interp.eval(interp.env, expanded.value()).await;
            match result {
                Ok(value) => {
                    interp.flush_stdout().await;
                    println!(" = {}", interp.display(value));
                }
                Err(e) => eprintln!("Evaluation failed: {e}"),
            }
        }
        Err(e) => eprintln!("Expansion failed: {e}"),
    }
}

const HISTORY_FILENAME: &str = ".scheme.history";

async fn repl(interp: &Scheme) {
    let mut rl = DefaultEditor::new().expect("Failed to init REPL.");

    if rl.load_history(HISTORY_FILENAME).is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                let mut parser = Parser::from_string(&line);
                let expr = parser.read(interp).await;
                match expr {
                    Ok(expr) => eval_expr(interp, expr.value()).await,
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

    /// Display memory stats when the garbage collector runs.
    #[arg(long)]
    verbose_gc: bool,

    /// Initial heap size in number of objects.
    #[arg(short = 's', long, default_value_t = 256 * 1024)]
    heap_size: usize,

    /// Expressions to be evaluated after files are loaded.
    #[arg(short = 'e', long, action=ArgAction::Append)]
    exprs: Vec<String>,
}

#[tokio::main]
async fn main() {
    let arg = <Arg as clap::Parser>::parse();
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let options = SchemeOptions::new()
                .set_init_scheme(arg.init)
                .set_debug_macro(arg.debug_macro)
                .set_verbose_gc(arg.verbose_gc)
                .set_heap_size(arg.heap_size);
            let interp = Scheme::new(&options).await;
            for file in &arg.files {
                println!("Loading {}", file);
                match interp.load(file).await {
                    Err(e) => {
                        panic!("Failed to load {}: {}", file.to_string(), e);
                    }
                    _ => {}
                }
            }
            for expr in &arg.exprs {
                let _ = interp.eval_string(expr).await;
            }
            repl(&interp).await;
        })
        .await
}
