use std::{cell::RefCell, rc::Rc, time::Duration};

use tokio::{
    task::{JoinHandle, spawn_local},
    time::sleep,
};

use crate::{
    heap::ForeignObject,
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

type SchemeTask = JoinHandle<Result<Value, SchemeError>>;

fn primitive_spawn<'a>(interp: &'a Scheme, env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    if args.len() != 1 {
        return Box::pin(async move {
            Err(SchemeError::ArgCountError(format!(
                "Expected 1 arg, but got {}",
                args.len()
            )))
        });
    }
    let interp: &'static Scheme = unsafe { std::mem::transmute(interp) };
    let thunk = args[0].clone();
    let handle: SchemeTask = spawn_local(async move { interp.apply(env, thunk, vec![]).await });

    Box::pin(async move {
        let handle = Box::new(RefCell::new(Some(handle)));
        let pointer = handle as Box<dyn std::any::Any>;
        let foreign = ForeignObject {
            pointer,
            type_name: "thread",
        };
        EvalResult::done(interp.alloc_foreign(Rc::new(foreign)).value())
    })
}

fn primitive_join<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let foreign = interp.to_foreign(args[0])?;
        let cell = foreign
            .pointer
            .downcast_ref::<RefCell<Option<SchemeTask>>>()
            .ok_or(SchemeError::ImplementationError(format!(
                "Invalid downcast !"
            )))?;
        let handle = cell
            .borrow_mut()
            .take()
            .ok_or(SchemeError::AsyncError(format!("handle already joined.")))?;
        match handle.await {
            Ok(result) => result.map(|value| EvalResult::Done(value)),
            Err(e) => Err(SchemeError::AsyncError(format!(
                "Failed to join handle: {e}"
            ))),
        }
    })
}

pub fn register(interp: &Scheme) {
    interp.define_async_primitive("yield", primitive_yield);
    interp.define_async_primitive("sleep", primitive_sleep);
    interp.define_async_primitive("spawn", primitive_spawn);
    interp.define_async_primitive("join", primitive_join);
}
