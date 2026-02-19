use std::{cell::RefCell, process, rc::Rc};

use crate::{
    env::Env,
    interp::Interp,
    types::{EvalResult, SchemeError, Value},
};

fn primitive_gc(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    _args: &[Value],
) -> Result<EvalResult, SchemeError> {
    interp.gc(Some(env));
    EvalResult::done(Value::Nil)
}

fn primitive_heap_stats(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    _args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let stats = interp.heap.borrow().stats();
    println!("Total slots: {}", stats.total_slots);
    println!(" Live slots: {}", stats.live_slots);
    println!(" Free slots: {}", stats.free_slots);
    println!("  Next slot: {}", stats.next_slot);
    println!("    Symbols: {}", stats.symbol_count);
    EvalResult::done(Value::Nil)
}

fn primitive_debug(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let output = interp.get_output_port()?;
    if let Some(ref mut port) = *output.port.borrow_mut() {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                write!(port, " ")?;
            }
            write!(port, "{}", interp.display(*arg))?;
        }
        writeln!(port)?;
        port.flush()?;
    }
    EvalResult::done(Value::Boolean(true))
}

fn primitive_load(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    let mut retval = Value::Nil;
    for arg in args {
        let filename = interp.to_string(*arg)?.to_string();
        retval = interp.load(&filename)?;
    }
    EvalResult::done(retval)
}

fn primitive_quit(
    _interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    extract_args!(args, 1, exit_code: Number);
    match i32::try_from(*exit_code) {
        Ok(code) => process::exit(code),
        Err(_) => Err(SchemeError::OverflowError(format!(
            "Overflow while converting {} to i32",
            exit_code
        ))),
    }
}

pub fn register(interp: &Interp) {
    interp.define_primitive("gc", primitive_gc);
    interp.define_primitive("heap-stats", primitive_heap_stats);
    interp.define_primitive("debug", primitive_debug);
    interp.define_primitive("load", primitive_load);
    interp.define_primitive("quit", primitive_quit);
    interp.define_primitive("exit", primitive_quit);
}
