use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::rc::Rc;

use crate::env::Env;
use crate::heap::{self, Handle, Heap, PrimitiveFn};
use crate::heap::{Apply, Closure, HeapObject, Keyword, OutputPort, Vector};
use crate::markset::MarkSet;
use crate::parser::Parser;
use crate::types::{DisplayWrapper, EvalResult, GcId, Number, SchemeError, SchemeObject, Value};

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
                heap.raw_intern_symbol("append"),
                heap.raw_intern_symbol("list"),
                heap.raw_intern_symbol("quasiquote"),
                heap.raw_intern_symbol("unquote"),
                heap.raw_intern_symbol("unquote-splicing"),
            )
        };
        let interp = Self {
            heap: heap_handle,
            env: env_handle,

            input_stack: RefCell::new(vec![]),
            output_stack: RefCell::new(vec![]),

            list: list.expect("list symbol").value(),
            append: append.expect("append symbol").value(),
            quasiquote: quasiquote.expect("quasiquote symbol").value(),
            unquote: unquote.expect("unquote symbol").value(),
            unquote_splicing: unquote_splicing.expect("unquote-splicing symbol").value(),
        };
        interp.init();
        interp
    }

    fn init_io(&self) {
        // Sets up stdin as the default input port.
        let boxed_reader: Box<dyn BufRead> = Box::new(BufReader::new(std::io::stdin()));
        let input_port = self.alloc_input_port(Rc::new(RefCell::new(Some(boxed_reader))));
        self.input_stack.borrow_mut().push(input_port.value());

        // Sets up stdout as the default output port.
        let boxed_writer: Box<dyn Write> = Box::new(BufWriter::new(std::io::stdout()));
        let output_port = self.alloc_output_port(Rc::new(RefCell::new(Some(boxed_writer))));
        self.output_stack.borrow_mut().push(output_port.value())
    }

    fn alloc_with_retry<F>(&self, mut alloc_fn: F) -> Handle
    where
        F: FnMut(&mut Heap) -> Result<Handle, SchemeError>,
    {
        if let Ok(result) = alloc_fn(&mut self.heap.borrow_mut()) {
            return result;
        }
        println!("GC Trigered: Out of memory.");
        // TODO Garbage collect
        alloc_fn(&mut self.heap.borrow_mut()).expect("Out of memory after GC.")
    }

    pub fn intern_symbol(&self, name: &str) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_intern_symbol(name))
    }

    pub fn alloc_pair(&self, car: Value, cdr: Value) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_pair(car, cdr))
    }

    pub fn alloc_list(&self, items: &[Value]) -> Handle {
        items
            .into_iter()
            .rfold(Handle::from_value(Value::Nil), |acc, val| {
                self.alloc_pair(*val, acc.value())
            })
    }

    pub fn alloc_list_from_handles(&self, items: &[Handle]) -> Handle {
        items
            .into_iter()
            .rfold(Handle::from_value(Value::Nil), |acc, val| {
                self.alloc_pair(val.value(), acc.value())
            })
    }

    pub fn alloc_list_with_cdr(&self, items: &[Value], cdr: Value) -> Handle {
        items
            .into_iter()
            .rfold(Handle::from_value(cdr), |acc, val| {
                self.alloc_pair(*val, acc.value())
            })
    }

    pub fn alloc_list_with_cdr_from_handles(&self, items: &[Handle], cdr: Value) -> Handle {
        items
            .into_iter()
            .rfold(Handle::from_value(cdr), |acc, val| {
                self.alloc_pair(val.value(), acc.value())
            })
    }

    pub fn alloc_string(&self, s: impl Into<String>) -> Handle {
        let owned = s.into();
        self.alloc_with_retry(|heap| heap.raw_alloc_string(owned.clone()))
    }

    pub fn alloc_primitive(&self, func: PrimitiveFn) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_primitive(func))
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

    pub fn alloc_input_port(&self, input: Rc<RefCell<Option<Box<dyn BufRead>>>>) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_input_port(input.clone()))
    }

    pub fn alloc_output_port(&self, output: Rc<RefCell<Option<Box<dyn Write>>>>) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_output_port(output.clone()))
    }

    pub fn alloc_output_string_port(&self) -> Handle {
        self.alloc_with_retry(|heap| heap.raw_alloc_output_string_port())
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

    pub fn mark(&self, marks: &mut MarkSet) {
        self.heap.borrow().mark(self, marks);
        for port in self.input_stack.borrow().iter() {
            port.mark(self, marks);
        }
        for port in self.output_stack.borrow().iter() {
            port.mark(self, marks);
        }
    }

    pub fn define(&self, name: &str, value: Value) -> Value {
        let symbol = self.intern_symbol(name);
        self.env.borrow_mut().define(symbol.id(), value);
        symbol.value()
    }

    pub fn define_primitive(&self, name: &str, func: heap::PrimitiveFn) {
        // TODO Retry on alloc failure
        let prim = self
            .heap
            .borrow_mut()
            .raw_alloc_primitive(func)
            .expect("Should garbage collect!!");
        self.define(name, prim.value());
    }

    fn init(&self) {
        self.init_io();
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

    pub fn last(&self, car: Value) -> Result<Value, SchemeError> {
        self.heap.borrow().last(car)
    }

    pub fn setcdr(&self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        self.heap.borrow_mut().setcdr(id, value)
    }

    pub fn lookup(&self, name: &str) -> Handle {
        self.intern_symbol(name)
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

    pub fn expand_quasiquote(&self, expr: Value) -> Result<Handle, SchemeError> {
        match expr {
            Value::Object(id) => {
                let obj = { self.heap.borrow().get(id).clone() };
                match obj {
                    HeapObject::Pair(car, cdr) if car == self.unquote => {
                        Ok(Handle::Value(self.to_car(cdr)?))
                    }
                    HeapObject::Pair(..) => {
                        let mut p = expr;
                        let mut args = vec![Handle::Value(self.append)];
                        loop {
                            if let Some((car, cdr)) = self.is_pair(p) {
                                if let Some(spliced) = self.is_splicing(car)? {
                                    args.push(Handle::Value(spliced))
                                } else {
                                    args.push(self.list_from_handle(self.expand_quasiquote(car)?)?);
                                }
                                p = cdr;
                            } else if p == Value::Nil {
                                return Ok(self.alloc_list_from_handles(&args));
                            } else {
                                return Ok(self.alloc_list_with_cdr_from_handles(&args, p));
                            }
                        }
                    }
                    _ => self.quote(expr),
                }
            }
            _ => self.quote(expr),
        }
    }

    fn expand_macro(&self, func: Value, args: Value) -> Result<Handle, SchemeError> {
        let arg_handles = self.fold_list(args, Vec::new(), |mut acc, arg| {
            acc.push(self.expand(arg)?);
            Ok(acc)
        })?;
        let args: Vec<Value> = arg_handles.iter().map(|h| h.value()).collect();
        let expansion = match func.apply(self, self.env.clone(), args)? {
            EvalResult::Done(value) => value,
            EvalResult::Continuation(next_env, next_expr) => self.eval(next_env, next_expr)?,
        };
        Ok(Handle::Value(expansion))
    }

    fn get_macro(&self, id: GcId) -> Option<Value> {
        // This function's purpose is to limit the scope of env borrowing.
        self.env.borrow().macros.get(&id).cloned()
    }

    pub fn expand(&self, expr: Value) -> Result<Handle, SchemeError> {
        if let Some((car, cdr)) = self.is_pair(expr) {
            if let Value::Object(id) = car
                && id == 8
            {
                Ok(Handle::Value(expr))
            } else if let Value::Object(id) = car
                && let Some(func) = self.get_macro(id)
            {
                let expansion = self.expand_macro(func, cdr)?;
                Ok(self.expand(expansion.value())?)
            } else {
                let mut updated = false;
                let items = self.fold_list(expr, vec![], |mut acc, item| {
                    let expansion = self.expand(item)?;
                    updated = updated || expansion.value() != item;
                    acc.push(expansion);
                    Ok(acc)
                });
                if updated {
                    Ok(self.alloc_list_from_handles(&items?))
                } else {
                    Ok(Handle::Value(expr))
                }
            }
        } else {
            Ok(Handle::Value(expr))
        }
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Value, SchemeError> {
        let mut parser = Parser::from_file(path)?;
        let mut retval = Value::Eof;
        loop {
            let handle = parser.read(self)?;
            match handle {
                Handle::Value(Value::Eof) => return Ok(retval),
                _ => {
                    let expansion = self.expand(handle.value())?;
                    retval = self.eval(self.env.clone(), expansion.value())?;
                }
            }
        }
    }
}
