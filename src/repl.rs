use rustyline::{DefaultEditor, error::ReadlineError};
use tokio::sync::{mpsc::Sender, oneshot};

const HISTORY_FILENAME: &str = ".scheme.history";

pub struct ReplRequest {
    pub input: String,
    pub reply_to: oneshot::Sender<String>,
}

fn repl_round_trip(tx: &Sender<ReplRequest>, input: String) -> String {
    let (reply_tx, reply_rx) = oneshot::channel();
    if let Err(e) = tx.blocking_send(ReplRequest {
        input,
        reply_to: reply_tx,
    }) {
        return format!("Failed to send to interpreter: {e}");
    }
    match futures::executor::block_on(reply_rx) {
        Ok(response) => response,
        Err(e) => format!("Interpreter thread died: {e}"),
    }
}

pub fn repl(tx: Sender<ReplRequest>) {
    let mut rl = DefaultEditor::new().expect("Failed to init REPL.");

    if rl.load_history(HISTORY_FILENAME).is_err() {
        println!("No previous history.");
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                println!("= {}", repl_round_trip(&tx, line));
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
