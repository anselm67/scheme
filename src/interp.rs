use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process;
use std::rc::Rc;

use crate::env::Env;
use crate::heap::{Apply, Closure, HeapObject, Keyword, OutputPort, Vector};
use crate::markset::MarkSet;
use crate::parser::Parser;
use crate::types::{DisplayWrapper, EvalResult, GcId, Number, SchemeError, SchemeObject, Value};
use crate::{check_arity, check_min_arity, extract_args, heap};

pub struct Interp {
    pub heap: RefCell<heap::Heap>,
    pub env: Rc<RefCell<crate::env::Env>>,

    // IO ports
    input_stack: RefCell<Vec<Value>>,
    output_stack: RefCell<Vec<Value>>,

    // Some symbols we want to keeep track of:
    append: Value,
    list: Value,
    quasiquote: Value,
    unquote: Value,
    unquote_splicing: Value,
}

struct PortGuard<'a> {
    stack: &'a RefCell<Vec<Value>>,
}

impl<'a> Drop for PortGuard<'a> {
    fn drop(&mut self) {
        self.stack.borrow_mut().pop();
    }
}

impl Interp {
    pub fn new() -> Self {
        let global_env = crate::env::Env {
            macros: HashMap::new(),
            bindings: HashMap::new(),
            parent: None,
        };
        let env_handle = Rc::new(RefCell::new(global_env));
        let heap_handle = RefCell::new(heap::Heap::new(8192));
        let (append, list, quasiquote, unquote, unquote_splicing) = {
            let mut heap = heap_handle.borrow_mut();
            (
                heap.intern_symbol("append"),
                heap.intern_symbol("list"),
                heap.intern_symbol("quasiquote"),
                heap.intern_symbol("unquote"),
                heap.intern_symbol("unquote-splicing"),
            )
        };
        let interp = Self {
            heap: heap_handle,
            env: env_handle,

            input_stack: RefCell::new(vec![]),
            output_stack: RefCell::new(vec![]),

            list: list,
            append: append,
            quasiquote: quasiquote,
            unquote: unquote,
            unquote_splicing: unquote_splicing,
        };
        interp.init_io();
        interp.init();
        interp
    }

    fn init_io(&self) {
        let mut heap = self.heap.borrow_mut();

        // Sets up stdin as the default input port.
        let boxed_reader: Box<dyn BufRead> = Box::new(BufReader::new(std::io::stdin()));
        let input_port = heap.alloc_input_port(Rc::new(RefCell::new(Some(boxed_reader))));
        self.input_stack.borrow_mut().push(input_port);

        // Sets up stdout as the default output port.
        let boxed_writer: Box<dyn Write> = Box::new(BufWriter::new(std::io::stdout()));
        let output_port = heap.alloc_output_port(Rc::new(RefCell::new(Some(boxed_writer))));
        self.output_stack.borrow_mut().push(output_port)
    }

    pub fn with_input_port<F, T>(&self, value: Value, thunk: F) -> Result<T, SchemeError>
    where
        F: FnOnce() -> Result<T, SchemeError>,
    {
        let input = self.to_input_port(value)?;
        if input.borrow().is_none() {
            Err(SchemeError::IOError(format!(
                "Attempt to read from a closed output port."
            )))
        } else {
            self.input_stack.borrow_mut().push(value);
            let _guard = PortGuard {
                stack: &self.input_stack,
            };
            thunk()
        }
    }

    pub fn with_output_port<F, T>(&self, value: Value, thunk: F) -> Result<T, SchemeError>
    where
        F: FnOnce() -> Result<T, SchemeError>,
    {
        let output = self.to_output_port(value)?;
        if output.port.borrow().is_none() {
            Err(SchemeError::IOError(format!(
                "Attempt to write to a closed output port."
            )))
        } else {
            self.output_stack.borrow_mut().push(value);
            let _guard = PortGuard {
                stack: &self.input_stack,
            };
            thunk()
        }
    }

    pub fn get_input_port_as_value(&self) -> Result<Value, SchemeError> {
        if let Some(value) = self.input_stack.borrow().last() {
            Ok(*value)
        } else {
            panic!("No input port on the input stack!");
        }
    }

    pub fn get_input_port(&self) -> Result<Rc<RefCell<Option<Box<dyn BufRead>>>>, SchemeError> {
        if let Some(value) = self.input_stack.borrow().last() {
            self.to_input_port(*value)
        } else {
            panic!("No input port on the input stack!");
        }
    }

    pub fn get_output_port_as_value(&self) -> Result<Value, SchemeError> {
        if let Some(value) = self.output_stack.borrow().last() {
            Ok(*value)
        } else {
            panic!("No output port on the input stack!");
        }
    }
    pub fn get_output_port(&self) -> Result<OutputPort, SchemeError> {
        if let Some(value) = self.output_stack.borrow().last() {
            self.to_output_port(*value)
        } else {
            panic!("No output port on the input stack!");
        }
    }

    fn mark(&self, marks: &mut MarkSet) {
        self.heap.borrow().mark(self, marks);
        for port in self.input_stack.borrow().iter() {
            port.mark(self, marks);
        }
        for port in self.output_stack.borrow().iter() {
            port.mark(self, marks);
        }
    }

    pub fn define(&self, name: &str, value: Value) -> Value {
        let symbol = self.heap.borrow_mut().intern_symbol(name);
        if let Value::Object(id) = symbol {
            self.env.borrow_mut().define(id, value);
        }
        symbol
    }

    pub fn define_primitive(&self, name: &str, func: heap::PrimitiveFn) {
        let prim = self.heap.borrow_mut().alloc_primitive(func);
        self.define(name, prim);
    }

    fn init(&self) {
        self.define_primitive("eval", primitive_eval);
        self.define_primitive("apply", primitive_apply);
        self.define_primitive("expand", primitive_expand);
        self.define_primitive("eq?", primitive_eq);
        self.define_primitive("equal?", primitive_equal);
        self.define_primitive("error", primitive_error);
        self.define_primitive("with-exception-handler", primitive_with_exception_handler);
        self.define_primitive("procedure?", primitive_procedure_p);
        self.define_primitive("closure?", primitive_closure_p);
        self.define_primitive("closure->body", primitive_closure_body);
        self.define("#t", Value::Boolean(true));
        self.define("#f", Value::Boolean(false));

        // Initialize symbol functions.
        self.define_primitive("symbol?", primitive_symbol_p);

        // IO primitive functions.

        // Initialize system primitive functions.
        self.define_primitive("gc", primitive_gc);
        self.define_primitive("heap-stats", primitive_heap_stats);
        self.define_primitive("debug", primitive_debug);
        self.define_primitive("load", primitive_load);
        self.define_primitive("quit", primitive_quit);
        self.define_primitive("exit", primitive_quit);

        crate::primitives::register_all(self);
    }

    pub fn fold_list<T, F>(&self, list: Value, init: T, mut func: F) -> Result<T, SchemeError>
    where
        F: FnMut(T, Value) -> Result<T, SchemeError>,
    {
        let mut p = list;
        let mut acc = init;
        while let Some((car, cdr)) = self.is_pair(p) {
            acc = func(acc, car)?;
            p = cdr;
        }
        Ok(acc)
    }

    pub fn lookup(&self, name: &str) -> Value {
        self.heap.borrow_mut().intern_symbol(name)
    }

    pub fn eval(&self, env: Rc<RefCell<Env>>, expr: Value) -> Result<Value, SchemeError> {
        let mut current_expr = expr;
        let mut current_env = env;
        loop {
            match current_expr.eval(self, current_env)? {
                EvalResult::Done(value) => return Ok(value),
                EvalResult::Continuation(next_env, next_expr) => {
                    current_expr = next_expr;
                    current_env = next_env;
                }
            }
        }
    }

    pub fn apply(
        &self,
        env: Rc<RefCell<Env>>,
        f: Value,
        args: Vec<Value>,
    ) -> Result<Value, SchemeError> {
        match f.apply(self, env.clone(), args)? {
            EvalResult::Done(value) => return Ok(value),
            EvalResult::Continuation(next_env, next_expr) => self.eval(next_env, next_expr),
        }
    }

    pub fn display(&self, obj: Value) -> String {
        let wrapper = DisplayWrapper {
            obj: &obj,
            interp: self,
        };
        wrapper.to_string()
    }

    pub fn is_nil(&self, value: Value) -> bool {
        matches!(value, Value::Nil)
    }

    pub fn is_list(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            matches!(self.heap.borrow().get(id), HeapObject::Pair(..))
        } else if matches!(value, Value::Nil) {
            true
        } else {
            false
        }
    }

    pub fn is_null(&self, value: Value) -> bool {
        matches!(value, Value::Nil)
    }

    pub fn is_integer(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(n @ Number::Int(_)) => Some(n),
            _ => None,
        }
    }

    pub fn to_integer(&self, value: Value) -> Result<i64, SchemeError> {
        match value {
            Value::Number(Number::Int(i)) => Ok(i),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an Number::Int, but got a {}.",
                value.type_name()
            ))),
        }
    }

    pub fn is_float(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(f @ Number::Float(_)) => Some(f),
            _ => None,
        }
    }

    pub fn is_number(&self, value: Value) -> Option<Number> {
        match value {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    pub fn is_char(&self, value: Value) -> Option<u8> {
        match value {
            Value::Char(ch) => Some(ch),
            _ => None,
        }
    }

    pub fn to_char(&self, value: Value) -> Result<char, SchemeError> {
        match value {
            Value::Char(ch) => Ok(ch as char),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Char got a {}",
                value.type_name()
            ))),
        }
    }

    pub fn is_string(&self, value: Value) -> Option<Ref<'_, String>> {
        if let Some(id) = self.is_object(value) {
            Ref::filter_map(self.heap.borrow(), |h| {
                if let HeapObject::String(string) = h.get(id) {
                    Some(string)
                } else {
                    None
                }
            })
            .ok()
        } else {
            None
        }
    }

    pub fn to_string(&self, value: Value) -> Result<Ref<'_, String>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        Ref::filter_map(heap, |h| {
            if let HeapObject::String(string) = h.get(id) {
                Some(string)
            } else {
                None
            }
        })
        .map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected a String, but got a {}",
                value.type_name()
            ))
        })
    }

    pub fn to_string_mut(&self, value: Value) -> Result<RefMut<'_, String>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow_mut();
        RefMut::filter_map(heap, |h| {
            if let HeapObject::String(string) = h.get_mut(id) {
                Some(string)
            } else {
                None
            }
        })
        .map_err(|_| {
            SchemeError::TypeError(format!(
                "Expected a String, but got a {}",
                value.type_name()
            ))
        })
    }

    pub fn is_closure(&self, value: Value) -> Option<Ref<'_, Closure>> {
        if let Some(id) = self.is_object(value) {
            Ref::filter_map(self.heap.borrow(), |h| match h.get(id) {
                HeapObject::Closure(closure) => Some(closure.as_ref()),
                HeapObject::NaryClosure(closure) => Some(closure.as_ref()),
                _ => None,
            })
            .ok()
        } else {
            None
        }
    }

    pub fn to_closure(&self, value: Value) -> Result<Ref<'_, Closure>, SchemeError> {
        if let Some(closure) = self.is_closure(value) {
            Ok(closure)
        } else {
            Err(SchemeError::TypeError(format!(
                "Expected a Closre, but got a {}",
                value.type_name()
            )))
        }
    }

    pub fn is_vector(&self, value: Value) -> Option<Ref<'_, Vector>> {
        if let Some(id) = self.is_object(value) {
            Ref::filter_map(self.heap.borrow(), |h| {
                if let HeapObject::Vector(vector) = h.get(id) {
                    Some(vector)
                } else {
                    None
                }
            })
            .ok()
        } else {
            None
        }
    }

    pub fn to_vector(&self, value: Value) -> Result<Ref<'_, Vector>, SchemeError> {
        if let Some(vector) = self.is_vector(value) {
            Ok(vector)
        } else {
            Err(SchemeError::TypeError(format!(
                "Expected a Vector, but got a {}",
                value.type_name()
            )))
        }
    }

    pub fn to_input_port(
        &self,
        value: Value,
    ) -> Result<Rc<RefCell<Option<Box<dyn BufRead>>>>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        match heap.get(id) {
            HeapObject::InputPort(input) => Ok(input.clone()),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an InputPort, but got a {}",
                value.type_name()
            ))),
        }
    }

    pub fn to_output_port(&self, value: Value) -> Result<OutputPort, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        match heap.get(id) {
            HeapObject::OutputPort(output) => Ok(output.clone()),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an OutputPort, but got a {}",
                value.type_name()
            ))),
        }
    }

    pub fn is_object(&self, value: Value) -> Option<GcId> {
        match value {
            Value::Object(id) => Some(id),
            _ => None,
        }
    }

    pub fn to_object(&self, value: Value) -> Result<GcId, SchemeError> {
        match value {
            Value::Object(id) => Ok(id),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an Object, got a {}",
                value.type_name()
            ))),
        }
    }

    pub fn is_pair(&self, value: Value) -> Option<(Value, Value)> {
        if let Some(id) = self.is_object(value) {
            match self.heap.borrow().get(id) {
                HeapObject::Pair(car, cdr) => Some((*car, *cdr)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn to_pair(&self, value: Value) -> Result<(Value, Value), SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::Pair(car, cdr) => Ok((*car, *cdr)),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {}.",
                value.type_name()
            ))),
        }
    }

    pub fn to_car(&self, value: Value) -> Result<Value, SchemeError> {
        let (car, _) = self.to_pair(value)?;
        Ok(car)
    }

    pub fn is_symbol(&self, value: Value) -> Option<GcId> {
        if let Some(id) = self.is_object(value) {
            if matches!(self.heap.borrow().get(id), HeapObject::Symbol(_)) {
                return Some(id);
            }
        }
        None
    }

    pub fn to_symbol(&self, value: Value) -> Result<GcId, SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::Symbol(_) => Ok(id),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Symbol, but got a {}.",
                value.type_name()
            ))),
        }
    }

    pub fn is_procedure(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            let heap = self.heap.borrow();
            match heap.get(id) {
                HeapObject::Closure(_) => true,
                HeapObject::NaryClosure(_) => true,
                HeapObject::Primitive(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn quote(&self, obj: Value) -> Result<Value, SchemeError> {
        let value = &[Value::Object(Keyword::Quote as usize), obj];
        Ok(self.heap.borrow_mut().alloc_list(value))
    }

    pub fn quasiquote(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.quasiquote, obj]))
    }

    pub fn unquote(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.unquote, obj]))
    }

    pub fn unquote_splicing(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.unquote_splicing, obj]))
    }

    pub fn list(&self, obj: Value) -> Result<Value, SchemeError> {
        let mut heap = self.heap.borrow_mut();
        Ok(heap.alloc_list(&[self.list, obj]))
    }

    fn is_splicing(&self, value: Value) -> Result<Option<Value>, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(value)
            && car == self.unquote_splicing
        {
            let (cadr, _) = self.to_pair(cdr)?;
            Ok(Some(cadr))
        } else {
            Ok(None)
        }
    }

    pub fn expand_quasiquote(&self, expr: Value) -> Result<Value, SchemeError> {
        match expr {
            Value::Object(id) => {
                let obj = { self.heap.borrow().get(id).clone() };
                match obj {
                    HeapObject::Pair(car, cdr) if car == self.unquote => self.to_car(cdr),
                    HeapObject::Pair(..) => {
                        let mut p = expr;
                        let mut args = vec![self.append];
                        loop {
                            if let Some((car, cdr)) = self.is_pair(p) {
                                if let Some(spliced) = self.is_splicing(car)? {
                                    args.push(spliced)
                                } else {
                                    args.push(self.list(self.expand_quasiquote(car)?)?);
                                }
                                p = cdr;
                            } else if p == Value::Nil {
                                let mut heap = self.heap.borrow_mut();
                                return Ok(heap.alloc_list(&args));
                            } else {
                                let mut heap = self.heap.borrow_mut();
                                return Ok(heap.alloc_list_with_cdr(&args, p));
                            }
                        }
                    }
                    _ => self.quote(expr),
                }
            }
            _ => self.quote(expr),
        }
    }

    fn expand_macro(&self, func: Value, args: Value) -> Result<Value, SchemeError> {
        let args = self.fold_list(args, Vec::new(), |mut acc, arg| {
            acc.push(self.expand(arg)?);
            Ok(acc)
        });
        let expansion = match func.apply(self, self.env.clone(), args?)? {
            EvalResult::Done(value) => value,
            EvalResult::Continuation(next_env, next_expr) => self.eval(next_env, next_expr)?,
        };
        Ok(expansion)
    }

    fn get_macro(&self, id: GcId) -> Option<Value> {
        // This function's purpose is to limit the scope of env borrowing.
        self.env.borrow().macros.get(&id).cloned()
    }

    pub fn expand(&self, expr: Value) -> Result<Value, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(expr) {
            if let Value::Object(id) = car
                && id == 8
            {
                Ok(expr)
            } else if let Value::Object(id) = car
                && let Some(func) = self.get_macro(id)
            {
                Ok(self.expand(self.expand_macro(func, cdr)?)?)
            } else {
                let mut updated = false;
                let items = self.fold_list(expr, vec![], |mut acc, item| {
                    let expansion = self.expand(item)?;
                    updated = updated || expansion != item;
                    acc.push(expansion);
                    Ok(acc)
                });
                if updated {
                    let expansion = {
                        let mut heap = self.heap.borrow_mut();
                        heap.alloc_list(&items?).clone()
                    };
                    Ok(expansion)
                } else {
                    Ok(expr)
                }
            }
        } else {
            Ok(expr)
        }
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Value, SchemeError> {
        let mut parser = Parser::from_file(path)?;
        let mut retval = Value::Eof;
        loop {
            match parser.read(self)? {
                Value::Eof => return Ok(retval),
                expr => {
                    retval = self.expand(expr)?;
                    retval = self.eval(self.env.clone(), retval)?;
                }
            }
        }
    }
}

fn primitive_eval(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(interp.eval(env, args[0])?)
}

fn primitive_apply(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    use crate::heap::Apply;
    check_min_arity!(args, 2);
    let func = args[0];
    let (last, firsts) = args[1..]
        .split_last()
        .ok_or(SchemeError::ArgCountError(format!(
            "Expected at least 2 args, got {}",
            args.len()
        )))?;
    let all_args = interp.fold_list(*last, firsts.to_vec(), |mut acc, arg| {
        acc.push(arg);
        Ok(acc)
    })?;
    func.apply(interp, env, all_args)
}

fn primitive_expand(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(interp.expand(args[0])?)
}

fn primitive_equal(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::done(Value::Boolean(args[0].is_equal(interp, &args[1])))
}

fn primitive_eq(
    _interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    EvalResult::done(Value::Boolean(args[0] == args[1]))
}

fn primitive_error(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let string = interp.to_string(args[0])?.to_string();
    Err(SchemeError::UserError(string))
}

fn primitive_with_exception_handler(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 2);
    let handler = args[0];
    let thunk = args[1];
    match interp.apply(env.clone(), thunk, vec![]) {
        Ok(value) => EvalResult::done(value),
        Err(e) => {
            let (label, message) = e.get_infos();
            let string = interp
                .heap
                .borrow_mut()
                .alloc_string(format!("[{}]: {}", label, message));
            EvalResult::done(interp.apply(env.clone(), handler, vec![string])?)
        }
    }
}

fn primitive_procedure_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_procedure(args[0])))
}

fn primitive_closure_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_closure(args[0]).is_some()))
}

fn primitive_closure_body(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    let body = {
        let closure = interp.to_closure(args[0])?;
        closure.get_body()
    };
    let mut heap = interp.heap.borrow_mut();
    EvalResult::done(heap.alloc_list(&body))
}

fn primitive_symbol_p(
    interp: &Interp,
    _env: Rc<RefCell<Env>>,
    args: &[Value],
) -> Result<EvalResult, SchemeError> {
    check_arity!(args, 1);
    EvalResult::done(Value::Boolean(interp.is_symbol(args[0]).is_some()))
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

/**
 * IO primitives.
 */
fn primitive_gc(
    interp: &Interp,
    env: Rc<RefCell<Env>>,
    _args: &[Value],
) -> Result<EvalResult, SchemeError> {
    // Marks all reachable objects from the environment and the heap's symbols.
    let len = interp.heap.borrow().len();
    let mut marks = MarkSet::new(len);

    interp.mark(&mut marks);
    env.borrow().mark(interp, &mut marks);

    // Collects all unreachable objects lying in the heap.
    let mut heap = interp.heap.borrow_mut();
    let collected = heap.sweep(&marks);

    println!(
        "gc: marked {} /{} objects, collected {}.",
        marks.count(),
        len,
        collected
    );

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
