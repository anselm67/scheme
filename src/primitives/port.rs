use std::{cell::RefCell, rc::Rc};

use tokio::{
    fs::File,
    io::{
        AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
        BufWriter,
    },
};

use crate::{
    interp::Scheme,
    parser::Parser,
    types::{EvalFuture, EvalResult, SchemeError, Value},
};

fn primitive_input_port_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_input_port(args[0])))
}

fn primitive_output_port_p(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_output_port(args[0])))
}

fn primitive_open_input_file<'a>(
    interp: &'a Scheme,
    _env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let filename = interp.to_string(args[0])?.borrow().to_string();
        let file = File::open(&filename)
            .await
            .map_err(|_| SchemeError::FileNotFound(format!("Can't open file {}", filename)))?;
        let reader = BufReader::new(file);
        let boxed_reader: Box<dyn AsyncBufRead + Unpin> = Box::new(reader);
        let input = Rc::new(RefCell::new(Some(boxed_reader)));
        EvalResult::done(interp.alloc_input_port(input).value())
    })
}

fn primitive_open_input_string(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let text = { interp.to_string(args[0])?.borrow().as_bytes().to_vec() };
    let cursor = std::io::Cursor::new(text);
    let boxed_reader: Box<dyn AsyncBufRead + Unpin> = Box::new(cursor);
    let input = Rc::new(RefCell::new(Some(boxed_reader)));
    EvalResult::done(interp.alloc_input_port(input).value())
}

fn primitive_close_input_port(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let input = interp.to_input_port(args[0])?;
    let _ = input.borrow_mut().take();
    EvalResult::done(Value::Nil)
}

fn primitive_read_char<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        let mut input = interp.get_input_port()?;
        check_arity_range!(args, 0, 1);
        if args.len() == 1 {
            input = interp.to_input_port(args[0])?;
        }
        let mut borrow = input.borrow_mut();
        if let Some(ref mut reader) = *borrow {
            let mut buf = [0u8; 1];
            match reader.read_exact(&mut buf).await {
                Ok(_) => EvalResult::char(buf[0] as char),
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    EvalResult::done(Value::Eof)
                }
                Err(e) => Err(SchemeError::IOError(format!("Read error {}", e))),
            }
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to read from closed input port."
            )))
        }
    })
}

fn primitive_read_line<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        let mut input = interp.get_input_port()?;
        check_arity_range!(args, 0, 1);
        if args.len() == 1 {
            input = interp.to_input_port(args[0])?;
        }
        let mut borrow = input.borrow_mut();
        if let Some(ref mut reader) = *borrow {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Result::Ok(0) => EvalResult::done(Value::Eof),
                Result::Ok(_) => EvalResult::done(interp.alloc_string(&line).value()),
                Result::Err(e) => Err(SchemeError::from(e)),
            }
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to read from closed input port."
            )))
        }
    })
}

fn primitive_peek_char<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        let mut input = interp.get_input_port()?;
        check_arity_range!(args, 0, 1);
        if args.len() == 1 {
            input = interp.to_input_port(args[0])?;
        }
        let mut borrow = input.borrow_mut();
        if let Some(ref mut reader) = *borrow {
            match reader.fill_buf().await {
                Ok(b) if b.is_empty() => EvalResult::done(Value::Eof),
                Ok(b) => EvalResult::char(b[0] as char),
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    EvalResult::done(Value::Eof)
                }
                Err(e) => Err(SchemeError::IOError(format!("Read error {}", e))),
            }
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to read from closed input port."
            )))
        }
    })
}

fn primitive_eof_object(
    _interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::bool(args[0] == Value::Eof)
}

fn primitive_open_output_file<'a>(
    interp: &'a Scheme,
    _env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let filename = interp.to_string(args[0])?;
        let filename = filename.borrow();
        let file = File::create(filename.clone()).await.map_err(|e| {
            SchemeError::FileNotFound(format!(
                "Couldn't open file {} for writing: {}",
                filename, e
            ))
        })?;
        let writer: Box<dyn AsyncWrite + Unpin> = Box::new(BufWriter::new(file));
        let output = RefCell::new(Some(writer));
        EvalResult::done(interp.alloc_output_port(&output).value())
    })
}

fn primitive_open_output_string(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 0);
    let port = interp.alloc_output_string_port();
    EvalResult::done(port.value())
}

fn primitive_get_output_string(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity_range!(args, 0, 1);
    let mut output = interp.get_output_port()?;
    if args.len() > 0 {
        output = interp.to_output_port(args[0])?;
    }
    if let Some(buffer) = &output.string_buffer {
        let bytes = buffer.borrow();
        let string = String::from_utf8_lossy(&bytes).into_owned();
        EvalResult::done(interp.alloc_string(string).value())
    } else {
        Err(SchemeError::TypeError(format!(
            "This OutputPort isn't backed by a string."
        )))
    }
}

fn primitive_close_output_port<'a>(
    interp: &'a Scheme,
    _env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 1);
        let output = interp.to_output_port(args[0])?;
        let mut guard = output.port.lock().await;
        let _ = guard.take();
        EvalResult::done(Value::Nil)
    })
}

fn primitive_flush_output_port<'a>(
    interp: &'a Scheme,
    _env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity_range!(args, 0, 1);
        let mut output = interp.get_output_port()?;
        if args.len() > 0 {
            output = interp.to_output_port(args[0])?;
        }
        let mut guard = output.port.lock().await;
        if let Some(ref mut writer) = *guard {
            writer.flush().await?;
            EvalResult::done(Value::Nil)
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to write to a closed output port."
            )))
        }
    })
}

fn primitive_with_input_port<'a>(
    interp: &'a Scheme,
    env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 2);
        interp
            .with_input_port(args[0], || async {
                EvalResult::done(interp.apply(env, args[1], vec![]).await?)
            })
            .await
    })
}

fn primitive_with_output_port<'a>(
    interp: &'a Scheme,
    env: Value,
    args: &'a [Value],
) -> EvalFuture<'a> {
    Box::pin(async move {
        check_arity!(args, 2);
        interp
            .with_output_port(args[0], || async {
                EvalResult::done(interp.apply(env, args[1], vec![]).await?)
            })
            .await
    })
}

fn primitive_current_input_port(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 0);
    EvalResult::done(interp.get_input_port_as_value()?)
}

fn primitive_current_output_port(
    interp: &Scheme,
    _env: Value,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 0);
    EvalResult::done(interp.get_output_port_as_value()?)
}

fn primitive_write_char<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        let mut output = interp.get_output_port()?;
        check_arity_range!(args, 1, 2);
        let ch;
        if args.len() == 1 {
            ch = interp.to_char(args[0])?;
        } else {
            ch = interp.to_char(args[0])?;
            output = interp.to_output_port(args[1])?;
        }

        let mut guard = output.port.lock().await;
        if let Some(ref mut writer) = *guard {
            writer.write_u8(ch as u8).await?;
            EvalResult::done(Value::Nil)
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to write to a closed output port."
            )))
        }
    })
}

fn primitive_read<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move {
        let mut input = interp.get_input_port()?;
        if args.len() == 1 {
            input = interp.to_input_port(args[0])?;
        }
        let mut borrow = input.borrow_mut();
        if let Some(ref mut reader) = *borrow {
            let mut parser = Parser::from_borrowed(reader.as_mut());
            let expr = parser.read(interp).await?;
            EvalResult::done(expr.value())
        } else {
            Err(SchemeError::IOError(format!(
                "Attempt to read from closed input port."
            )))
        }
    })
}

async fn primitive_output<F>(
    interp: &Scheme,
    args: &[Value],
    func: F,
) -> Result<EvalResult, SchemeError>
where
    F: FnOnce(Value) -> String,
{
    check_arity_range!(args, 1, 2);
    let obj = args[0];
    let mut output = interp.get_output_port()?;
    if args.len() == 2 {
        output = interp.to_output_port(args[1])?;
    }
    let mut guard = output.port.lock().await;
    if let Some(ref mut port) = *guard {
        port.write_all(func(obj).as_bytes()).await?;
        port.flush().await?;
    }
    EvalResult::bool(true)
}

fn primitive_display<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move { primitive_output(interp, args, |obj| interp.display(obj)).await })
}

fn primitive_write<'a>(interp: &'a Scheme, _env: Value, args: &'a [Value]) -> EvalFuture<'a> {
    Box::pin(async move { primitive_output(interp, args, |obj| interp.write(obj)).await })
}

pub fn register(interp: &Scheme) {
    interp.define_primitive("input-port?", primitive_input_port_p);
    interp.define_primitive("output-port?", primitive_output_port_p);
    interp.define_async_primitive("open-input-file", primitive_open_input_file);
    interp.define_primitive("open-input-string", primitive_open_input_string);
    interp.define_primitive("close-input-port", primitive_close_input_port);
    interp.define_async_primitive("open-output-file", primitive_open_output_file);
    interp.define_primitive("open-output-string", primitive_open_output_string);
    interp.define_primitive("get-output-string", primitive_get_output_string);
    interp.define_async_primitive("close-output-port", primitive_close_output_port);
    interp.define_async_primitive("read", primitive_read);
    interp.define_async_primitive("read-char", primitive_read_char);
    interp.define_async_primitive("read-line", primitive_read_line);
    interp.define_async_primitive("peek-char", primitive_peek_char);
    interp.define_primitive("eof-object?", primitive_eof_object);
    interp.define_async_primitive("write-char", primitive_write_char);
    interp.define_async_primitive("flush-output-port", primitive_flush_output_port);
    interp.define_async_primitive("with-output-port", primitive_with_output_port);
    interp.define_async_primitive("with-input-port", primitive_with_input_port);
    interp.define_primitive("current-output-port", primitive_current_output_port);
    interp.define_primitive("current-input-port", primitive_current_input_port);
    interp.define_async_primitive("write", primitive_write);
    interp.define_async_primitive("display", primitive_display);
}
