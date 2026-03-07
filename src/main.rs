use clap::ArgAction;
use scheme::parser::Parser;
use scheme::repl::{ReplRequest, repl};
use scheme::types::Value;

use scheme::interp::{Scheme, SchemeOptions};
use tokio::sync::mpsc::{self};
use tokio::task::{spawn_blocking, spawn_local};

async fn eval_expr(interp: &Scheme, expr: Value) -> String {
    let expansion = interp.expand(expr).await;
    match expansion {
        Ok(expanded) => {
            if interp.debug_macro {
                eprintln!("expanded => {}", interp.display(expanded.value()));
            }
            let result = interp.eval(interp.env, expanded.value()).await;
            match result {
                Ok(value) => interp.display(value),
                Err(e) => format!("Evaluation failed: {e}"),
            }
        }
        Err(e) => format!("Expansion failed: {e}"),
    }
}

async fn eval_text(interp: &Scheme, text: &str) -> String {
    let mut parser = Parser::from_string(text);
    let expr = parser.read(interp).await;
    match expr {
        Ok(expr) => eval_expr(interp, expr.value()).await,
        Err(e) => format!("Error: {:?}", e).to_string(),
    }
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
    let (tx, mut rx) = mpsc::channel::<ReplRequest>(100);

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let worker = spawn_local(async move {
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
                while let Some(request) = rx.recv().await {
                    let output_string = eval_text(&interp, &request.input).await;
                    let _ = request.reply_to.send(output_string);
                }
                println!("Bye !");
            });
            let tx_clone = tx.clone();
            spawn_blocking(move || {
                repl(tx_clone);
            });
            drop(tx);
            let _ = worker.await;
        })
        .await;
}
