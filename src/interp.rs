use std::cell::{Cell, Ref, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use async_recursion::async_recursion;
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

use crate::env::Env;
use crate::heap::{self, AsyncPrimitiveFn, ForeignObject, Handle, Heap, PrimitiveFn};
use crate::heap::{Apply, Closure, HeapObject, Keyword, OutputPort};
use crate::markset::MarkSet;
use crate::parser::Parser;
use crate::types::{EvalResult, GcId, Number, SchemeError, SchemeObject, Value};

pub struct Scheme {
    pub heap: RefCell<heap::Heap>,
    pub env: Value,

    // IO ports
    input_stack: RefCell<Vec<Value>>,
    output_stack: RefCell<Vec<Value>>,

    // Misc control flags
    pub debug_macro: bool,
    verbose_gc: bool,

    // Some symbols we want to keeep track of:
    append: Value,
    list: Value,
    quote: Value,
    quasiquote: Value,
    unquote: Value,
    unquote_splicing: Value,
    apply: Value,
    vector: Value,

    empty_string: Handle,
}

struct PortGuard<'a> {
    stack: &'a RefCell<Vec<Value>>,
}

impl<'a> Drop for PortGuard<'a> {
    fn drop(&mut self) {
        self.stack.borrow_mut().pop();
    }
}

enum OutputWrapperKind {
    ForWrite,
    ForDisplay,
}

struct OutputWrapper<'a> {
    kind: OutputWrapperKind,
    obj: &'a Value,
    interp: &'a Scheme,
}

impl<'a> std::fmt::Display for OutputWrapper<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            OutputWrapperKind::ForDisplay => self.obj.display(self.interp, f),
            OutputWrapperKind::ForWrite => self.obj.write(self.interp, f),
        }
    }
}

pub struct SchemeOptions {
    heap_size: usize,
    init_scheme: bool,
    debug_macro: bool,
    verbose_gc: bool,
}

impl SchemeOptions {
    pub fn new() -> Self {
        Self {
            heap_size: 256 * 1024,
            init_scheme: true,
            debug_macro: false,
            verbose_gc: false,
        }
    }
    pub fn set_init_scheme(mut self, init: bool) -> Self {
        self.init_scheme = init;
        self
    }

    pub fn set_heap_size(mut self, heap_size: usize) -> Self {
        self.heap_size = heap_size;
        self
    }

    pub fn set_debug_macro(mut self, debug_macro: bool) -> Self {
        self.debug_macro = debug_macro;
        self
    }

    pub fn set_verbose_gc(mut self, verbose_gc: bool) -> Self {
        self.verbose_gc = verbose_gc;
        self
    }
}

impl Scheme {
    pub async fn new(options: &SchemeOptions) -> Self {
        let heap_handle = RefCell::new(heap::Heap::new(options.heap_size));
        let global_env = crate::env::Env {
            macros: HashMap::new(),
            bindings: HashMap::new(),
            parent: None,
        };
        let env = heap_handle
            .borrow_mut()
            .raw_alloc_env(Rc::new(RefCell::new(global_env)))
            .expect("Failed to allocate global env.");

        let (append, list, quote, quasiquote, unquote, unquote_splicing, apply, vector) = {
            let mut heap = heap_handle.borrow_mut();
            (
                heap.raw_intern_symbol("append")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("list")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("quote")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("quasiquote")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("unquote")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("unquote-splicing")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("apply")
                    .expect("raw_intern_symbol at init")
                    .1,
                heap.raw_intern_symbol("vector")
                    .expect("raw_intern_symbol at init")
                    .1,
            )
        };
        let empty_string = {
            let mut heap = heap_handle.borrow_mut();
            heap.raw_alloc_string("").expect("Init empty string.")
        };
        let interp = Self {
            heap: heap_handle,
            env: env.value(),
            input_stack: RefCell::new(vec![]),
            output_stack: RefCell::new(vec![]),

            debug_macro: options.debug_macro,
            verbose_gc: options.verbose_gc,

            list: list.value(),
            append: append.value(),
            quote: quote.value(),
            quasiquote: quasiquote.value(),
            unquote: unquote.value(),
            unquote_splicing: unquote_splicing.value(),
            apply: apply.value(),
            vector: vector.value(),

            empty_string: empty_string,
        };
        interp.init(options).await;
        interp
    }

    fn init_io(&self) {
        // Sets up stdin as the default input port.
        let boxed_reader: Box<dyn AsyncBufRead + Unpin> =
            Box::new(BufReader::new(tokio::io::stdin()));
        let input_port = self.alloc_input_port(Rc::new(RefCell::new(Some(boxed_reader))));
        self.input_stack.borrow_mut().push(input_port.value());

        // Sets up stdout as the default output port.
        let boxed_writer: Box<dyn AsyncWrite + Unpin> =
            Box::new(BufWriter::new(tokio::io::stdout()));
        let output_port = self.alloc_output_port(&RefCell::new(Some(boxed_writer)));
        self.output_stack.borrow_mut().push(output_port.value())
    }

    pub async fn flush_stdout(&self) {
        let stack = self.output_stack.borrow();
        let port_value = *stack.first().expect("Output stack should never be empty!");
        let output = self
            .to_output_port(port_value)
            .expect("stdout should be a valid output port.");
        let mut guard = output.port.borrow_mut();
        if let Some(writer) = guard.as_deref_mut() {
            let _ = writer.flush().await;
        }
    }

    async fn init_scheme(&self) {
        let text = include_str!("scheme/macros.scm");
        let mut parser = Parser::from_string(text);
        if let Err(e) = self.load_from_parser(&mut parser).await {
            panic!("Init from scheme/macros.scm failed: {}", e);
        }
    }

    pub fn handle(&self, value: Value) -> Handle {
        self.heap.borrow().handle(value)
    }

    fn alloc_with_retry<F, R>(&self, mut alloc_fn: F) -> R
    where
        F: FnMut(&mut Heap) -> Result<R, SchemeError>,
    {
        if let Ok(result) = alloc_fn(&mut self.heap.borrow_mut()) {
            return result;
        }
        self.gc(None);
        alloc_fn(&mut self.heap.borrow_mut()).expect("Out of memory after GC.")
    }

    pub fn intern_symbol(&self, name: &str) -> (Rc<str>, Handle) {
        self.alloc_with_retry(|heap| heap.raw_intern_symbol(name))
    }

    pub fn alloc_env(&self, env: Rc<RefCell<Env>>) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_env(env.clone()))
    }

    pub fn alloc_pair(&self, car: Value, cdr: Value) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_pair(car, cdr))
    }

    pub fn alloc_pair_from_handles(&self, car: Handle, cdr: Handle) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_pair(car.value(), cdr.value()))
    }

    pub fn alloc_list(&self, items: &[Value]) -> Handle {
        items
            .into_iter()
            .rfold(self.handle(Value::Nil), |acc, val| {
                self.alloc_pair(*val, acc.value())
            })
    }

    pub fn alloc_list_from_handles(&self, items: &[Handle]) -> Handle {
        items
            .into_iter()
            .rfold(self.handle(Value::Nil), |acc, val| {
                self.alloc_pair(val.value(), acc.value())
            })
    }

    pub fn alloc_list_with_cdr(&self, items: &[Value], cdr: Value) -> Handle {
        items.into_iter().rfold(self.handle(cdr), |acc, val| {
            self.alloc_pair(*val, acc.value())
        })
    }

    pub fn alloc_list_with_cdr_from_handles(&self, items: &[Handle], cdr: Value) -> Handle {
        items.into_iter().rfold(self.handle(cdr), |acc, val| {
            self.alloc_pair(val.value(), acc.value())
        })
    }

    pub fn alloc_string(&self, s: impl Into<String>) -> Handle {
        let owned = s.into();
        self.alloc_with_retry(|heap| heap.raw_alloc_string(owned.clone()))
    }

    pub fn alloc_primitive(&self, name: Rc<str>, func: PrimitiveFn) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_primitive(name.clone(), func))
    }

    pub fn alloc_async_primitive(&self, name: Rc<str>, func: AsyncPrimitiveFn) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_async_primitive(name.clone(), func))
    }

    pub fn alloc_foreign(&self, foreign: Rc<ForeignObject>) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_foreign(foreign.clone()))
    }

    pub fn alloc_closure(&self, closure: Closure) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_closure(closure.clone()))
    }

    pub fn alloc_nary_closure(&self, closure: Closure) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_nary_closure(closure.clone()))
    }

    pub fn alloc_vector(&self, items: &[Value]) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_vector(items))
    }

    pub fn alloc_vector_from_handles(&self, items: &[Handle]) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_vector_from_handles(items))
    }

    pub fn alloc_input_port(
        &self,
        input: Rc<RefCell<Option<Box<dyn AsyncBufRead + Unpin>>>>,
    ) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_input_port(input.clone()))
    }

    pub fn alloc_output_port(
        &self,
        output: &RefCell<Option<Box<dyn AsyncWrite + Unpin>>>,
    ) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_output_port(&output))
    }

    pub fn alloc_output_string_port(&self) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_output_string_port())
    }

    pub async fn with_input_port<F, Fut, T>(&self, value: Value, thunk: F) -> Result<T, SchemeError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, SchemeError>>,
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
            thunk().await
        }
    }

    pub async fn with_output_port<F, Fut, T>(
        &self,
        value: Value,
        thunk: F,
    ) -> Result<T, SchemeError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, SchemeError>>,
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
            thunk().await
        }
    }

    pub fn get_input_port_as_value(&self) -> Result<Value, SchemeError> {
        if let Some(value) = self.input_stack.borrow().last() {
            Ok(*value)
        } else {
            panic!("No input port on the input stack!");
        }
    }

    pub fn get_input_port(
        &self,
    ) -> Result<Rc<RefCell<Option<Box<dyn AsyncBufRead + Unpin>>>>, SchemeError> {
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
    pub fn get_output_port(&self) -> Result<Rc<OutputPort>, SchemeError> {
        if let Some(value) = self.output_stack.borrow().last() {
            self.to_output_port(*value)
        } else {
            panic!("No output port on the input stack!");
        }
    }

    pub fn mark(&self, marks: &mut MarkSet) {
        self.heap.borrow().mark(self, marks);
        for port in self.input_stack.borrow().iter() {
            port.mark(self, marks);
        }
        for port in self.output_stack.borrow().iter() {
            port.mark(self, marks);
        }
    }

    pub fn define(&self, symbol: Value, value: Value) -> Value {
        let id = self
            .to_object(symbol)
            .expect("define can only define a symbol.");
        let env = self.to_env(self.env);
        env.borrow_mut().define(id, value);
        symbol
    }

    pub fn define_from_string(&self, name: &str, value: Value) {
        let (_, handle) = self.intern_symbol(name);
        self.define(handle.value(), value);
    }

    pub fn define_primitive(&self, name: &str, func: PrimitiveFn) {
        let (name, handle) = self.intern_symbol(name);
        let prim = self.alloc_primitive(name.clone(), func);
        self.define(handle.value(), prim.value());
    }

    pub fn define_async_primitive(&self, name: &str, func: AsyncPrimitiveFn) {
        let (name, handle) = self.intern_symbol(name);
        let prim = self.alloc_async_primitive(name.clone(), func);
        self.define(handle.value(), prim.value());
    }

    async fn init(&self, options: &SchemeOptions) {
        self.init_io();
        crate::primitives::register_all(self);
        if options.init_scheme {
            self.init_scheme().await;
        }
    }

    // TODO This might not be needed in the end.
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

    pub async fn async_fold_list<T, F, Fut>(
        &self,
        list: Value,
        init: T,
        mut func: F,
    ) -> Result<T, SchemeError>
    where
        F: FnMut(T, Value) -> Fut,
        Fut: Future<Output = Result<T, SchemeError>>,
    {
        let mut p = list;
        let mut acc = init;
        while let Some((car, cdr)) = self.is_pair(p) {
            acc = func(acc, car).await?;
            p = cdr;
        }
        Ok(acc)
    }

    pub fn last(&self, car: Value) -> Result<Value, SchemeError> {
        self.heap.borrow().last(car)
    }

    pub fn setcar(&self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        self.heap.borrow_mut().setcar(id, value)
    }

    pub fn setcdr(&self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        self.heap.borrow_mut().setcdr(id, value)
    }

    pub fn lookup(&self, name: &str) -> Handle {
        self.intern_symbol(name).1
    }

    pub async fn eval(&self, env: Value, expr: Value) -> Result<Value, SchemeError> {
        let mut current_expr = self.handle(expr);
        let mut current_env = self.handle(env);

        loop {
            let result = current_expr.value().eval(self, current_env.value()).await?;

            match result {
                EvalResult::Done(value) => {
                    return Ok(value);
                }
                EvalResult::Continuation(next_env, next_expr) => {
                    current_expr = self.handle(next_expr);
                    current_env = self.handle(next_env);
                }
            }
        }
    }

    pub async fn apply(
        &self,
        env: Value,
        f: Value,
        args: Vec<Value>,
    ) -> Result<Value, SchemeError> {
        match f.apply(self, env, args).await? {
            EvalResult::Done(value) => return Ok(value),
            EvalResult::Continuation(next_env, next_expr) => self.eval(next_env, next_expr).await,
        }
    }

    pub fn display(&self, obj: Value) -> String {
        let wrapper = OutputWrapper {
            kind: OutputWrapperKind::ForDisplay,
            obj: &obj,
            interp: self,
        };
        wrapper.to_string()
    }

    pub fn write(&self, obj: Value) -> String {
        let wrapper = OutputWrapper {
            kind: OutputWrapperKind::ForWrite,
            obj: &obj,
            interp: self,
        };
        wrapper.to_string()
    }

    pub fn empty_string(&self) -> Value {
        self.empty_string.value()
    }

    pub fn is_nil(&self, value: Value) -> bool {
        matches!(value, Value::Nil)
    }

    /// This is very expensive!
    pub fn is_list(&self, value: Value) -> bool {
        if value == Value::Nil {
            return true;
        }
        if let Some(..) = self.is_pair(value) {
            let mut slow = value;
            let mut fast = value;
            loop {
                // fast moves two steps.
                for _ in 0..2 {
                    if fast == Value::Nil {
                        return true;
                    } else if let Some((_, next)) = self.is_pair(fast) {
                        fast = next;
                    } else {
                        return false;
                    }
                }
                // slow moves one step.
                if let Some((_, next)) = self.is_pair(slow) {
                    slow = next;
                } else {
                    return false;
                }
                // Checks for a circular list.
                if slow == fast {
                    return false;
                }
            }
        } else {
            return false;
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

    pub fn to_number(&self, value: Value) -> Result<Number, SchemeError> {
        match value {
            Value::Number(number) => Ok(number),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a Number, got a {}",
                value.type_name()
            ))),
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

    pub fn is_string(&self, value: Value) -> Option<Rc<RefCell<String>>> {
        if let Some(id) = self.is_object(value) {
            if let HeapObject::String(string) = self.heap.borrow().get(id) {
                Some(string.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn to_string(&self, value: Value) -> Result<Rc<RefCell<String>>, SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::String(string) => Ok(string.clone()),
            _ => Err(SchemeError::TypeError(format!(
                "Expected a String, but got a {}",
                value.type_name()
            ))),
        }
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

    pub fn is_vector(&self, value: Value) -> Option<Rc<RefCell<Vec<Value>>>> {
        if let Some(id) = self.is_object(value) {
            if let HeapObject::Vector(vector) = self.heap.borrow().get(id) {
                Some(vector.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn to_vector(&self, value: Value) -> Result<Rc<RefCell<Vec<Value>>>, SchemeError> {
        if let Some(vector) = self.is_vector(value) {
            Ok(vector)
        } else {
            Err(SchemeError::TypeError(format!(
                "Expected a Vector, but got a {}",
                value.type_name()
            )))
        }
    }

    pub fn is_input_port(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            let heap = self.heap.borrow();
            match heap.get(id) {
                HeapObject::InputPort(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn to_input_port(
        &self,
        value: Value,
    ) -> Result<Rc<RefCell<Option<Box<dyn AsyncBufRead + Unpin>>>>, SchemeError> {
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

    pub fn is_output_port(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            let heap = self.heap.borrow();
            match heap.get(id) {
                HeapObject::OutputPort(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn to_output_port(&self, value: Value) -> Result<Rc<OutputPort>, SchemeError> {
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

    pub fn is_foreign(&self, value: Value) -> bool {
        if let Some(id) = self.is_object(value) {
            let heap = self.heap.borrow();
            match heap.get(id) {
                HeapObject::Foreign(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn to_foreign(&self, value: Value) -> Result<Rc<ForeignObject>, SchemeError> {
        let id = self.to_object(value)?;
        let heap = self.heap.borrow();
        match heap.get(id) {
            HeapObject::Foreign(foreign) => Ok(foreign.clone()),
            _ => Err(SchemeError::TypeError(format!(
                "Expected an InputPort, but got a {}",
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

    pub fn is_env(&self, value: Value) -> Option<Rc<RefCell<Env>>> {
        if let Some(id) = self.is_object(value) {
            match self.heap.borrow().get(id) {
                HeapObject::Env(env) => Some(env.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn to_env(&self, value: Value) -> Rc<RefCell<Env>> {
        let id = self
            .to_object(value)
            .expect("to_env value isn't an Object.");
        match self.heap.borrow().get(id) {
            HeapObject::Env(env) => env.clone(),
            x => panic!("to_env {id} : got a {}.", x.type_name()),
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

    pub fn to_symbol_name(&self, value: Value) -> Result<String, SchemeError> {
        let id = self.to_object(value)?;
        match self.heap.borrow().get(id) {
            HeapObject::Symbol(name) => Ok(name.to_string()),
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

    fn peek_value(&self, value: Value) -> Result<Handle, SchemeError> {
        match value {
            Value::Object(id) => {
                let heap = self.heap.borrow();
                Ok(heap.handle(Value::int(id as i64)))
            }
            Value::Number(Number::Int(i)) => Ok(self.alloc_string(format!("Int({})", i))),
            Value::Number(Number::Float(f)) => Ok(self.alloc_string(format!("Float({})", f))),
            Value::Char(ch) => Ok(self.alloc_string(format!("Char({})", ch as char))),
            Value::Boolean(bool) => Ok(self.alloc_string(format!("Bool({})", bool))),
            Value::Nil => Ok(self.alloc_string(format!("()"))),
            Value::Unbound => Ok(self.alloc_string(format!("*unbound*"))),
            Value::Eof => Ok(self.alloc_string(format!("EoF"))),
        }
    }
    pub fn peek(&self, id: GcId) -> Result<Handle, SchemeError> {
        let obj = {
            let heap = self.heap.borrow();
            heap.checked_get(id)?.clone()
        };
        match obj {
            HeapObject::Pair(car, cdr) => {
                Ok(self.alloc_pair_from_handles(self.peek_value(car)?, self.peek_value(cdr)?))
            }
            HeapObject::Vector(vector) => {
                let items: Vec<Handle> = vector
                    .borrow()
                    .iter()
                    .map(|item| self.peek_value(*item))
                    .collect::<Result<Vec<_>, SchemeError>>()?;
                Ok(self.alloc_vector_from_handles(&items))
            }
            HeapObject::Env(env) => {
                let bindings: Vec<Handle> = env
                    .borrow()
                    .bindings
                    .iter()
                    .map(|(key, val)| {
                        let car = Handle::from_int(*key as i64);
                        self.peek_value(*val)
                            .and_then(|val| Ok(self.alloc_pair_from_handles(car, val)))
                    })
                    .collect::<Result<Vec<_>, SchemeError>>()?;
                Ok(self.alloc_vector_from_handles(&bindings))
            }
            HeapObject::FreeSlot(_) => Ok(self.alloc_string(format!("<free-slot {id}>"))),
            _ => Ok(self.heap.borrow().handle(Value::Object(id))),
        }
    }

    pub fn quote(&self, obj: Value) -> Result<Handle, SchemeError> {
        let value = &[Value::Object(Keyword::Quote as usize), obj];
        Ok(self.alloc_list(value))
    }

    pub fn quote_from_handle(&self, obj: Handle) -> Result<Handle, SchemeError> {
        let value = &[Value::Object(Keyword::Quote as usize), obj.value()];
        Ok(self.alloc_list(value))
    }

    pub fn quasiquote(&self, obj: Handle) -> Result<Handle, SchemeError> {
        Ok(self.alloc_list(&[self.quasiquote, obj.value()]))
    }

    pub fn unquote(&self, obj: Handle) -> Result<Handle, SchemeError> {
        Ok(self.alloc_list(&[self.unquote, obj.value()]))
    }

    pub fn unquote_splicing(&self, obj: Handle) -> Result<Handle, SchemeError> {
        Ok(self.alloc_list(&[self.unquote_splicing, obj.value()]))
    }

    pub fn list(&self, obj: Value) -> Result<Handle, SchemeError> {
        Ok(self.alloc_list(&[self.list, obj]))
    }

    pub fn list_from_handle(&self, obj: Handle) -> Result<Handle, SchemeError> {
        Ok(self.alloc_list(&[self.list, obj.value()]))
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

    fn expand_quasiquote(&self, expr: Value, depth: u32) -> Result<Handle, SchemeError> {
        match expr {
            Value::Object(id) => {
                let obj = { self.heap.borrow().get(id).clone() };
                match obj {
                    HeapObject::Pair(car, cdr) if car == self.unquote => {
                        let inner = self.to_car(cdr)?;
                        if depth == 0 {
                            Ok(self.handle(inner))
                        } else {
                            let expansion = self.expand_quasiquote(inner, depth - 1)?;
                            Ok(self.alloc_list_from_handles(&[
                                self.handle(self.list),
                                self.quote(self.unquote)?,
                                expansion,
                            ]))
                        }
                    }
                    HeapObject::Pair(car, cdr) if car == self.quasiquote => {
                        let expansion = self.expand_quasiquote(self.to_car(cdr)?, depth + 1)?;
                        Ok(self.alloc_list_from_handles(&[
                            self.handle(self.list),
                            self.quote(self.quasiquote)?,
                            expansion,
                        ]))
                    }
                    HeapObject::Pair(..) => {
                        let mut p = expr;
                        let mut args = vec![self.handle(self.append)];
                        loop {
                            if let Some((car, cdr)) = self.is_pair(p) {
                                if car == self.unquote {
                                    args.push(self.handle(self.to_car(cdr)?));
                                    return Ok(self.alloc_list_from_handles(&args));
                                } else if let Some(spliced) = self.is_splicing(car)? {
                                    args.push(self.handle(spliced))
                                } else {
                                    args.push(
                                        self.list_from_handle(self.expand_quasiquote(car, depth)?)?,
                                    );
                                }
                                p = cdr;
                            } else if p == Value::Nil {
                                return Ok(self.alloc_list_from_handles(&args));
                            } else {
                                args.push(self.expand_quasiquote(p, depth)?);
                                return Ok(self.alloc_list_from_handles(&args));
                            }
                        }
                    }
                    HeapObject::Vector(vector) => {
                        let mut items = vec![self.handle(self.append)];
                        for item in vector.borrow().iter() {
                            if let Some(spliced) = self.is_splicing(*item)? {
                                items.push(self.handle(spliced));
                            } else {
                                items.push(
                                    self.list_from_handle(self.expand_quasiquote(*item, depth)?)?,
                                );
                            }
                        }
                        let items = self.alloc_list_from_handles(&items);
                        let apply = self.alloc_list(&[self.apply, self.vector, items.value()]);
                        Ok(apply)
                    }
                    _ => self.quote(expr),
                }
            }
            _ => self.quote(expr),
        }
    }

    async fn expand_macro(&self, func: Value, args: Value) -> Result<Handle, SchemeError> {
        let args = self.fold_list(args, Vec::new(), |mut acc, arg| {
            acc.push(arg);
            Ok(acc)
        })?;
        let expansion = match func.apply(self, self.env, args).await? {
            EvalResult::Done(value) => self.handle(value),
            EvalResult::Continuation(next_env, next_expr) => {
                self.handle(self.eval(next_env, next_expr).await?)
            }
        };
        Ok(expansion)
    }

    pub fn get_macro(&self, id: GcId) -> Option<Value> {
        // This function's purpose is to limit the scope of env borrowing.
        let env = self.to_env(self.env);
        env.borrow().macros.get(&id).cloned()
    }

    #[async_recursion(?Send)]
    pub async fn expand(&self, expr: Value) -> Result<Handle, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(expr) {
            if car == self.quote {
                // We never expand quoted expressions!
                Ok(self.handle(expr))
            } else if car == self.quasiquote {
                self.expand_quasiquote(self.to_car(cdr)?, 0)
            } else if let Value::Object(id) = car
                && let Some(func) = self.get_macro(id)
            {
                if self.debug_macro {
                    println!("expand macro {}", self.display(cdr));
                }
                let expansion = self.expand_macro(func, cdr).await?;
                if self.debug_macro {
                    println!("expansion {}", self.display(expansion.value()));
                }
                Ok(self.expand(expansion.value()).await?)
            } else {
                let updated = Cell::new(false);
                let items = self
                    .async_fold_list(expr, vec![], |mut acc, item| {
                        let updated = &updated;
                        async move {
                            let expansion = self.expand(item).await?;
                            if expansion.value() != item {
                                updated.set(true);
                            }
                            acc.push(expansion);
                            Ok(acc)
                        }
                    })
                    .await;
                if updated.get() {
                    let expansion = self.alloc_list_from_handles(&items?);
                    Ok(expansion)
                } else {
                    Ok(self.handle(expr))
                }
            }
        } else {
            Ok(self.handle(expr))
        }
    }

    async fn load_from_parser<'a>(&self, parser: &mut Parser<'a>) -> Result<Value, SchemeError> {
        let mut retval = Value::Eof;
        loop {
            let handle = parser.read(self).await?;
            match handle.value() {
                Value::Eof => return Ok(retval),
                value => {
                    let expansion = self.expand(value).await?;
                    retval = self.eval(self.env, expansion.value()).await?;
                }
            }
        }
    }

    pub async fn load<P: AsRef<Path>>(&self, path: P) -> Result<Value, SchemeError> {
        let mut parser = Parser::from_file(path).await?;
        match self.load_from_parser(&mut parser).await {
            Err(error) => {
                let location = parser.last_location().clone();
                Err(SchemeError::At(location, Box::new(error)))
            }
            any => any,
        }
    }

    /// Runs the garbage collector.
    ///
    /// The garbage collector needs to run in one go: we can't afford to have
    /// any Scheme code run while it's at work. You've been warned!
    ///
    /// # Examples
    /// ```
    /// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    /// use scheme::interp::{Scheme, SchemeOptions};
    /// let interp = Scheme::new(&SchemeOptions::new()).await;
    /// &interp.gc(None);
    /// # });
    /// ```
    pub fn gc(&self, env: Option<Value>) {
        let len = { self.heap.borrow().len() };
        let mut marks = MarkSet::new(len);

        self.mark(&mut marks);
        let global_env = self.to_env(self.env);
        global_env.borrow().mark(self, &mut marks);
        if let Some(id) = env {
            let env = self.to_env(id);
            env.borrow().mark(self, &mut marks);
        }

        // Collects all unreachable objects lying in the heap.
        let mut heap = self.heap.borrow_mut();
        let collected = heap.sweep(&marks);

        if self.verbose_gc {
            println!(
                "gc: protected {}, marked {} /{} objects, collected {}.",
                heap.get_protected_count(),
                marks.count(),
                len,
                collected
            );
        }
    }

    /// Evaluates the Scheme code `text` in this interpreter.
    ///
    /// Parses `text` and evaluates its into a Value. If parsing fails or if
    /// the evaluation fails, returns a `SchemeError`
    /// # Examples
    /// ```
    /// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    /// use scheme::{interp::{Scheme, SchemeOptions}, types::Value};
    /// let interp = Scheme::new(&SchemeOptions::new()).await;
    ///
    /// let result = interp.eval_string("(+ 1 1)").await;
    /// assert_eq!(result.is_ok_and(|value| value == Value::int(2)), true);
    ///
    /// # });
    /// ```
    pub async fn eval_string(&self, text: &str) -> Result<Value, SchemeError> {
        let mut parser = Parser::from_string(text);
        let expr = parser.read(self).await?;
        let expanded = self.expand(expr.value()).await?;
        self.eval(self.env, expanded.value()).await
    }

    pub async fn blocking_eval_string(&self, expr: &str) -> Result<Value, SchemeError> {
        tokio::runtime::Handle::current().block_on(async { self.eval_string(expr).await })
    }
}
