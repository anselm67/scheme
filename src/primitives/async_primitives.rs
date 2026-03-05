use std::time::Duration;

use tokio::{task::spawn_local, time::sleep};

use crate::{
    interp::Scheme,
    types::{EvalFuture, EvalResult, SchemeError, Value},
};

fn primitive_yield<'a>(_interp: &'a Scheme, _env: Value, _args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        tokio::task::yield_now().await;
        EvalResult::done(Value::Nil)
    })
}

fn primitive_sleep<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let ms = interp.to_integer(args[0])?;
        sleep(Duration::from_millis(ms as u64)).await;
        EvalResult::bool(true)
    })
}

fn primitive_spawn<'a>(interp: &'a Scheme, env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    let args = args.to_vec();
    Box::pin(async move {
        let interp: &'static Scheme = unsafe { std::mem::transmute(interp) };
        let handle = spawn_local(async move {
            check_arity!(args, 1);
            let thunk = args[0];
            interp.apply(env, thunk, vec![]).await
        });
        match handle.await {
            Ok(result) => EvalResult::done(result?),
            Err(e) => {
                eprintln!("{e}");
                EvalResult::done(Value::Nil)
            }
        }
    })
}

pub fn register(interp: &Scheme) {
    interp.define_async_primitive("yield", primitive_yield);
    interp.define_async_primitive("sleep", primitive_sleep);
    interp.define_async_primitive("spawn", primitive_spawn);
}
