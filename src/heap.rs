use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc, task::Poll};
use tokio::io::{AsyncBufRead, AsyncWrite};

use crate::{
    check_arity, check_arity_range,
    env::Env,
    interp::Scheme,
    markset::MarkSet,
    types::{EvalFuture, EvalResult, GcId, Number, SchemeError, SchemeObject, Value},
};

pub type PrimitiveFn = fn(&Scheme, env: Value, &[Value]) -> Result<EvalResult, SchemeError>;
pub type AsyncPrimitiveFn = for<'a> fn(&'a Scheme, Value, &'a [Value]) -> EvalFuture<'a>;

enum PrimitiveKind {
    Sync(PrimitiveFn),
    Async(AsyncPrimitiveFn),
}
pub struct Primitive {
    kind: PrimitiveKind,
    name: Rc<str>,
}
#[derive(Clone)]
pub struct Closure {
    params: Box<[GcId]>,
    body: Box<[Value]>,
    tail: Value,
    env: Value,
}

impl Closure {
    pub fn get_body(&self) -> Vec<Value> {
        let mut body = self.body.to_vec();
        body.push(self.tail);
        body
    }
}

struct StringWriter {
    buffer: Rc<RefCell<Vec<u8>>>,
}

impl AsyncWrite for StringWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut b = self.buffer.borrow_mut();
        b.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub struct OutputPort {
    pub port: RefCell<Option<Box<dyn AsyncWrite + Unpin>>>,
    // The buffer is shared betwen the port and the writer.
    pub string_buffer: Option<Rc<RefCell<Vec<u8>>>>,
}

pub struct ForeignObject {
    pub pointer: Box<dyn std::any::Any>,
    pub type_name: &'static str,
}

#[derive(Clone)]
pub enum HeapObject {
    FreeSlot(GcId),
    Pair(Value, Value),
    Vector(Rc<RefCell<Vec<Value>>>),
    Symbol(Rc<str>),
    String(Rc<RefCell<String>>),
    Primitive(Rc<Primitive>),
    Closure(Box<Closure>),
    NaryClosure(Box<Closure>),
    InputPort(Rc<RefCell<Option<Box<dyn AsyncBufRead + Unpin>>>>),
    OutputPort(Rc<OutputPort>),
    Env(Rc<RefCell<Env>>),
    Foreign(Rc<ForeignObject>),
}

impl HeapObject {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::FreeSlot(_) => "FreeSlot",
            Self::Pair(..) => "Pair",
            Self::Vector(_) => "Vector",
            Self::Symbol(_) => "Symbol",
            Self::String(_) => "String",
            Self::Primitive(_) => "Primitive",
            Self::Closure(_) => "Closure",
            Self::NaryClosure(_) => "n-Closure",
            Self::InputPort(_) => "InputPort",
            Self::OutputPort(_) => "OutputPort",
            Self::Env(_) => "Env",
            Self::Foreign(_) => "Foreign",
        }
    }

    pub fn is_equal(&self, interp: &Scheme, other: &HeapObject) -> bool {
        match (self, other) {
            (HeapObject::FreeSlot(_), HeapObject::FreeSlot(_)) => false,
            (HeapObject::Pair(acar, acdr), HeapObject::Pair(bcar, bcdr)) => {
                acar.is_equal(interp, bcar) && acdr.is_equal(interp, bcdr)
            }
            (HeapObject::Vector(v1), HeapObject::Vector(v2)) => {
                let d1 = v1.borrow();
                let d2 = v2.borrow();
                d1.len() == d2.len() && d1.iter().zip(d2.iter()).all(|(a, b)| a.is_equal(interp, b))
            }
            (HeapObject::Symbol(a), HeapObject::Symbol(b)) => a == b,
            (HeapObject::String(a), HeapObject::String(b)) => a == b,
            (HeapObject::Primitive(p1), HeapObject::Primitive(p2)) => std::ptr::eq(p1, p2),
            (HeapObject::Closure(c1), HeapObject::Closure(c2)) => std::ptr::eq(c1, c2),
            (HeapObject::NaryClosure(p1), HeapObject::NaryClosure(p2)) => std::ptr::eq(p1, p2),
            _ => false,
        }
    }
}

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Keyword {
    If = 0,
    DefineBang = 1,
    Lambda = 2,
    Quote = 3,
    True = 4,
    False = 5,
    SetBang = 6,
    QuasiQuote = 7,
    DefineSyntax = 8,
}

fn extract_param_ids(interp: &Scheme, params: Value) -> Result<(Vec<GcId>, bool), SchemeError> {
    let mut ids = Vec::new();
    let mut p = params;
    let mut is_nary = false;

    if let Some(id) = interp.is_symbol(params) {
        ids.push(id);
        is_nary = true;
    } else {
        while let Some((car, cdr)) = interp.is_pair(p) {
            ids.push(interp.to_symbol(car)?);
            if interp.is_nil(cdr) {
                break;
            } else if interp.is_pair(cdr).is_some() {
                p = cdr;
            } else {
                is_nary = true;
                ids.push(interp.to_symbol(cdr)?);
                break;
            }
        }
    }
    Ok((ids, is_nary))
}

impl Keyword {
    fn from_id(id: GcId) -> Option<Keyword> {
        match id {
            0 => Some(Keyword::If),
            1 => Some(Keyword::DefineBang),
            2 => Some(Keyword::Lambda),
            3 => Some(Keyword::Quote),
            4 => Some(Keyword::True),
            5 => Some(Keyword::False),
            6 => Some(Keyword::SetBang),
            7 => Some(Keyword::QuasiQuote),
            8 => Some(Keyword::DefineSyntax),
            _ => None,
        }
    }

    fn eval<'a>(
        interp: &'a Scheme,
        env: Value,
        keyword: Keyword,
        args: &'a [Value],
    ) -> EvalFuture<'a> {
        Box::pin(async move {
            match keyword {
                Keyword::If => {
                    check_arity_range!(args, 2, 3);
                    let condition = interp.eval(env, args[0]).await?;
                    match condition {
                        Value::Boolean(false) => {
                            if args.len() > 2 {
                                Ok(EvalResult::Continuation(env, args[2]))
                            } else {
                                EvalResult::done(Value::Unbound)
                            }
                        }
                        _ => Ok(EvalResult::Continuation(env, args[1])),
                    }
                }
                Keyword::DefineBang => {
                    check_arity!(args, 2);
                    let symbol = interp.to_object(args[0])?;
                    let value = interp.eval(env, args[1]).await?;
                    let env = interp.to_env(env);
                    env.borrow_mut().define(symbol, value);
                    Ok(EvalResult::Done(Value::Nil))
                }
                Keyword::DefineSyntax => {
                    check_arity!(args, 2);
                    let symbol = interp.to_symbol(args[0])?;
                    let value = interp.eval(env, args[1]).await?;
                    let env = interp.to_env(env);
                    env.borrow_mut().define_syntax(symbol, value);
                    Ok(EvalResult::Done(Value::Nil))
                }
                Keyword::Lambda => match args {
                    [params_value, body @ .., tail] => {
                        let (params, is_nary) = extract_param_ids(interp, *params_value)?;
                        if is_nary {
                            Ok(EvalResult::Done(
                                interp
                                    .alloc_nary_closure(Closure {
                                        params: params.into_boxed_slice(),
                                        body: body.to_vec().into_boxed_slice(),
                                        tail: *tail,
                                        env: env,
                                    })
                                    .value(),
                            ))
                        } else {
                            Ok(EvalResult::Done(
                                interp
                                    .alloc_closure(Closure {
                                        params: params.into_boxed_slice(),
                                        body: body.to_vec().into_boxed_slice(),
                                        tail: *tail,
                                        env: env,
                                    })
                                    .value(),
                            ))
                        }
                    }
                    _ => Err(SchemeError::EvalError(format!(
                        "lambda expects at least 2 arguments, got {}",
                        args.len()
                    ))),
                },
                Keyword::Quote => {
                    check_arity!(args, 1);
                    Ok(EvalResult::Done(args[0]))
                }
                Keyword::QuasiQuote => Err(SchemeError::ImplementationError(format!(
                    "Keyword::QuasiQuote should never be evaluated, missing a call to expand()?"
                ))),
                Keyword::SetBang => {
                    check_arity!(args, 2);
                    let var = args[0];
                    let value = interp.eval(env, args[1]).await?;
                    if let Value::Object(var_id) = var {
                        Env::set_bang(interp, env, var_id, value)?;
                        Ok(EvalResult::Done(value))
                    } else {
                        Err(SchemeError::TypeError(
                            "set! first argument must be a variable".to_string(),
                        ))
                    }
                }
                _ => {
                    return Err(SchemeError::EvalError("not implemented".to_string()));
                }
            }
        })
    }
}

enum HandleKind {
    Id {
        id: GcId,
        protected_rc: Rc<RefCell<HashMap<GcId, usize>>>,
    },
    Value(Value),
}

pub struct Handle {
    inner: HandleKind,
}

impl Drop for Handle {
    fn drop(&mut self) {
        match &self.inner {
            HandleKind::Id { id, protected_rc } => {
                let mut protected = protected_rc.borrow_mut();
                if let Some(count) = protected.get_mut(&id) {
                    *count -= 1;
                    if *count == 0 {
                        protected.remove(&id);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Handle {
    fn from_id(id: GcId, protected_rc: Rc<RefCell<HashMap<GcId, usize>>>) -> Self {
        let mut protected = protected_rc.borrow_mut();
        let count = protected.entry(id).or_insert(0);
        *count += 1;
        Self {
            inner: HandleKind::Id {
                id,
                protected_rc: protected_rc.clone(),
            },
        }
    }

    fn from_value(value: Value) -> Self {
        Self {
            inner: HandleKind::Value(value),
        }
    }

    pub fn from_number(number: Number) -> Handle {
        Handle::from_value(Value::Number(number))
    }

    pub fn from_int(i: i64) -> Handle {
        Handle::from_value(Value::Number(Number::Int(i)))
    }

    pub fn value(&self) -> Value {
        match &self.inner {
            HandleKind::Id { id, .. } => Value::Object(*id),
            HandleKind::Value(value) => *value,
        }
    }

    pub fn id(&self) -> GcId {
        match &self.inner {
            HandleKind::Id { id, .. } => *id,
            _ => panic!("Requesting id of a Value Handle !"),
        }
    }
}

pub struct Heap {
    objects: Vec<HeapObject>,
    symbols: HashMap<Rc<str>, GcId>,
    protected: Rc<RefCell<HashMap<GcId, usize>>>,
    size: usize,
    free_slot: usize,
}

pub struct HeapStats {
    pub total_slots: usize,
    pub live_slots: usize,
    pub next_slot: usize,
    pub free_slots: usize,
    pub symbol_count: usize,
}

impl Heap {
    pub fn new(size: usize) -> Self {
        let mut heap = Self {
            objects: vec![HeapObject::FreeSlot(0); size],
            symbols: HashMap::new(),
            protected: Rc::new(RefCell::new(HashMap::new())),
            size: size,
            free_slot: 0,
        };
        // Chain all slots into free slots.
        // FreeSlot(i) if i >= size means we've reached the end.
        for i in 0..size {
            heap.objects[i] = HeapObject::FreeSlot(i + 1);
        }
        // Pre-intern keywords
        heap.intern_special_keywwords();
        heap
    }

    fn next_id(&mut self) -> Result<GcId, SchemeError> {
        if self.free_slot < self.size {
            let available_id = self.free_slot;
            if let HeapObject::FreeSlot(free_slot) = self.objects[self.free_slot] {
                self.free_slot = free_slot;
            } else {
                panic!(
                    "Free slot {} is occupied by a {} !",
                    self.free_slot,
                    self.objects[self.free_slot].type_name()
                )
            }
            return Ok(available_id);
        } else {
            Err(SchemeError::OutOfMemoryError(format!(
                "Out of memory, heap size {}",
                self.objects.len()
            )))
        }
    }

    fn handle_id(&self, id: GcId) -> Handle {
        Handle::from_id(id, self.protected.clone())
    }

    pub fn handle(&self, value: Value) -> Handle {
        match value {
            Value::Object(id) => Handle::from_id(id, self.protected.clone()),
            _ => Handle::from_value(value),
        }
    }

    pub fn get_protected_count(&self) -> usize {
        self.protected.borrow().len()
    }

    pub fn stats(&self) -> HeapStats {
        let free_count = self
            .objects
            .iter()
            .filter(|slot| matches!(slot, HeapObject::FreeSlot(_)))
            .count();
        HeapStats {
            total_slots: self.objects.len(),
            live_slots: self.size - free_count,
            next_slot: self.free_slot,
            free_slots: free_count,
            symbol_count: self.symbols.len(),
        }
    }

    fn intern_special_keywwords(&mut self) {
        let if_id = self.raw_intern_symbol("if");
        assert!(
            if_id.expect("init symbol").1.id() == Keyword::If as usize,
            "Keyword 'if' should have GcId 0"
        );
        let define_id = self.raw_intern_symbol("define!");
        assert!(
            define_id.expect("init symbol").1.id() == Keyword::DefineBang as usize,
            "Keyword 'define!' should have GcId 1"
        );
        let lambda_id = self.raw_intern_symbol("lambda");
        assert!(
            lambda_id.expect("init symbol").1.id() == Keyword::Lambda as usize,
            "Keyword 'lambda' should have GcId 2"
        );
        let quote_id = self.raw_intern_symbol("quote");
        assert!(
            quote_id.expect("init symbol").1.id() == Keyword::Quote as usize,
            "Keyword 'quote' should have GcId 3"
        );
        let true_id = self.raw_intern_symbol("#t");
        assert!(
            true_id.expect("init symbol").1.id() == Keyword::True as usize,
            "Keyword '#t' should have GcId 4"
        );
        let false_id = self.raw_intern_symbol("#f");
        assert!(
            false_id.expect("init symbol").1.id() == Keyword::False as usize,
            "Keyword '#f' should have GcId 5"
        );
        let set_bang_id = self.raw_intern_symbol("set!");
        assert!(
            set_bang_id.expect("init symbol").1.id() == Keyword::SetBang as usize,
            "Keyword 'set!' should have GcId 6"
        );
        let quasiquote_id = self.raw_intern_symbol("quasiquote");
        assert!(
            quasiquote_id.expect("init symbol").1.id() == Keyword::QuasiQuote as usize,
            "Keyword 'quasiquote' should have GcId 7"
        );
        let define_syntax_id = self.raw_intern_symbol("define-syntax");
        assert!(
            define_syntax_id.expect("init symbol").1.id() == Keyword::DefineSyntax as usize,
            "Keyword 'define-syntax' should have GcId 8"
        );
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn get(&self, id: GcId) -> &HeapObject {
        &self.objects[id]
    }

    pub fn checked_get(&self, id: GcId) -> Result<&HeapObject, SchemeError> {
        if id >= self.objects.len() {
            Err(SchemeError::IndexOutOfBounds(format!(
                "Object ids should be within 0..{}",
                self.objects.len()
            )))
        } else {
            Ok(&self.objects[id])
        }
    }

    fn get_mut(&mut self, id: GcId) -> &mut HeapObject {
        &mut self.objects[id]
    }

    pub fn raw_intern_symbol(&mut self, name: &str) -> Result<(Rc<str>, Handle), SchemeError> {
        if let Some((name, &id)) = self.symbols.get_key_value(name) {
            Ok((name.clone(), self.handle_id(id)))
        } else {
            let id: GcId = self.next_id()?;
            let name: Rc<str> = Rc::from(name);
            self.objects[id] = HeapObject::Symbol(name.clone());
            self.symbols.insert(name.clone(), id);
            Ok((name, self.handle_id(id)))
        }
    }

    pub fn raw_alloc_env(&mut self, env: Rc<RefCell<Env>>) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Env(env);
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_pair(&mut self, car: Value, cdr: Value) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Pair(car, cdr);
        Ok(self.handle_id(id))
    }

    pub fn last(&self, car: Value) -> Result<Value, SchemeError> {
        let mut tail = car;
        while let Value::Object(id) = tail {
            match self.get(id) {
                HeapObject::Pair(_, cdr) => {
                    if matches!(cdr, Value::Nil) {
                        return Ok(tail);
                    } else {
                        tail = *cdr;
                    }
                }
                _ => break,
            }
        }
        return Err(SchemeError::TypeError(format!(
            "Expected a Pair, but got a {}.",
            car.type_name()
        )));
    }

    pub fn setcar(&mut self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        match self.get_mut(id) {
            HeapObject::Pair(car, _) => {
                *car = value;
                Ok(value)
            }
            obj => Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {} instead.",
                obj.type_name()
            ))),
        }
    }

    pub fn setcdr(&mut self, id: GcId, value: Value) -> Result<Value, SchemeError> {
        match self.get_mut(id) {
            HeapObject::Pair(_, cdr) => {
                *cdr = value;
                Ok(value)
            }
            obj => Err(SchemeError::TypeError(format!(
                "Expected a Pair, but got a {} instead.",
                obj.type_name()
            ))),
        }
    }

    pub fn raw_alloc_string(&mut self, s: impl Into<String>) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::String(Rc::new(RefCell::new(s.into())));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_primitive(
        &mut self,
        name: Rc<str>,
        func: PrimitiveFn,
    ) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Primitive(Rc::new(Primitive {
            kind: PrimitiveKind::Sync(func),
            name: name.clone(),
        }));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_async_primitive(
        &mut self,
        name: Rc<str>,
        func: AsyncPrimitiveFn,
    ) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Primitive(Rc::new(Primitive {
            kind: PrimitiveKind::Async(func),
            name: name.clone(),
        }));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_closure(&mut self, closure: Closure) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Closure(Box::new(closure));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_nary_closure(&mut self, closure: Closure) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::NaryClosure(Box::new(closure));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_vector(&mut self, items: &[Value]) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Vector(Rc::new(RefCell::new(items.to_vec())));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_vector_from_handles(
        &mut self,
        items: &[Handle],
    ) -> Result<Handle, SchemeError> {
        let id: GcId = self.next_id()?;
        self.objects[id] = HeapObject::Vector(Rc::new(RefCell::new(
            items.iter().map(|h| h.value()).collect(),
        )));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_input_port(
        &mut self,
        input: Rc<RefCell<Option<Box<dyn AsyncBufRead + Unpin>>>>,
    ) -> Result<Handle, SchemeError> {
        let id = self.next_id()?;
        self.objects[id] = HeapObject::InputPort(input);
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_output_port(
        &mut self,
        output: &RefCell<Option<Box<dyn AsyncWrite + Unpin>>>,
    ) -> Result<Handle, SchemeError> {
        let id = self.next_id()?;
        let port_ref = output
            .borrow_mut()
            .take()
            .expect("Implementation error: expected a valid AsyncWrite.");
        self.objects[id] = HeapObject::OutputPort(Rc::new(OutputPort {
            port: RefCell::new(Some(port_ref)),
            string_buffer: None,
        }));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_output_string_port(&mut self) -> Result<Handle, SchemeError> {
        let id = self.next_id()?;
        let buffer = Rc::new(RefCell::new(Vec::<u8>::new()));
        let writer = StringWriter {
            buffer: buffer.clone(),
        };
        let boxed: Box<dyn AsyncWrite + Unpin> = Box::new(writer);
        self.objects[id] = HeapObject::OutputPort(Rc::new(OutputPort {
            port: RefCell::new(Some(boxed)),
            string_buffer: Some(buffer.clone()),
        }));
        Ok(self.handle_id(id))
    }

    pub fn raw_alloc_foreign(&mut self, foreign: Rc<ForeignObject>) -> Result<Handle, SchemeError> {
        let id = self.next_id()?;
        self.objects[id] = HeapObject::Foreign(foreign.clone());
        Ok(self.handle_id(id))
    }

    pub fn mark(&self, interp: &Scheme, marks: &mut MarkSet) {
        for id in self.symbols.values() {
            id.mark(interp, marks);
        }
        for id in self.protected.borrow().keys() {
            id.mark(interp, marks);
        }
    }

    fn make_free_slot(&mut self, id: GcId) {
        self.objects[id] = HeapObject::FreeSlot(self.free_slot);
        self.free_slot = id;
    }

    pub fn sweep(&mut self, marks: &MarkSet) -> usize {
        let mut count = 0;
        for id in 0..marks.len() {
            if !marks.is_marked(id) && !matches!(self.objects[id], HeapObject::FreeSlot(_)) {
                self.make_free_slot(id);
                count += 1;
            }
        }
        count
    }
}

pub trait Apply {
    fn apply<'a>(&'a self, interp: &'a Scheme, env: Value, args: Vec<Value>) -> EvalFuture<'a>;
}

impl Apply for Value {
    fn apply<'a>(&'a self, interp: &'a Scheme, env: Value, args: Vec<Value>) -> EvalFuture<'a> {
        Box::pin(async move {
            let obj = {
                let heap = interp.heap.borrow();
                match self {
                    Value::Object(id) => heap.get(*id).clone(),
                    _ => {
                        return Err(SchemeError::TypeError(format!(
                            "Attempted to apply a non-object value with type {}",
                            self.type_name()
                        )));
                    }
                }
            };

            match obj {
                HeapObject::Pair(car, _) => {
                    let func = interp.eval(env, car).await?;
                    let value = func.apply(interp, env, args).await?;
                    Ok(value)
                }
                HeapObject::Closure(closure) => {
                    check_arity!(args, closure.params.len());
                    let new_env = Env::extend(closure.env);
                    for (param_id, arg_value) in closure.params.iter().zip(args.iter()) {
                        new_env.borrow_mut().define(*param_id, *arg_value);
                    }
                    let env_handle = interp.alloc_env(new_env);
                    for expr in &closure.body {
                        interp.eval(env_handle.value(), *expr).await?;
                    }
                    Ok(EvalResult::Continuation(env_handle.value(), closure.tail))
                }
                HeapObject::NaryClosure(closure) => {
                    let new_env = Env::extend(closure.env);
                    let mut index = 0;
                    if args.len() < closure.params.len() - 1 {
                        return Err(SchemeError::ArgCountError(format!(
                            "Expected at least {} args, but got {}.",
                            closure.params.len() - 1,
                            args.len()
                        )));
                    }
                    while index < closure.params.len() - 1 {
                        new_env
                            .borrow_mut()
                            .define(closure.params[index], args[index]);
                        index += 1;
                    }
                    let rest = interp.alloc_list(&args[index..]);
                    new_env
                        .borrow_mut()
                        .define(closure.params[index], rest.value());
                    let env_handle = interp.alloc_env(new_env);
                    for expr in &closure.body {
                        interp.eval(env_handle.value(), *expr).await?;
                    }
                    Ok(EvalResult::Continuation(env_handle.value(), closure.tail))
                }
                HeapObject::Primitive(pr) => match pr.kind {
                    PrimitiveKind::Sync(func) => Ok(func(interp, env, &args)?),
                    PrimitiveKind::Async(func) => func(interp, env, &args).await,
                },
                HeapObject::FreeSlot(_) => {
                    panic!("Attempt to apply a FreeSlot!");
                }
                any => Err(SchemeError::TypeError(format!(
                    "Attempted to apply a non-primitive object with type {}",
                    any.type_name()
                ))),
            }
        })
    }
}

impl SchemeObject for GcId {
    fn eval<'a>(&'a self, interp: &'a Scheme, env: Value) -> EvalFuture<'a> {
        Box::pin(async move {
            let id = *self;
            let obj = {
                let heap = interp.heap.borrow();
                heap.get(id).clone()
            };

            match obj {
                HeapObject::Pair(car, cdr) => {
                    if let Value::Object(func_id) = car
                        && let Some(keyword) = Keyword::from_id(func_id)
                    {
                        // Special form handling - no args eval.
                        let arg_handles = interp.fold_list(cdr, Vec::new(), |mut acc, arg| {
                            acc.push(interp.handle(arg));
                            Ok(acc)
                        })?;
                        let args: Vec<Value> = arg_handles.iter().map(|h| h.value()).collect();
                        Keyword::eval(interp, env, keyword, &args).await
                    } else {
                        // Regular function call with arg eval.
                        let arg_handles = interp
                            .async_fold_list(cdr, Vec::new(), |mut acc, arg| async move {
                                let value = interp.eval(env, arg).await?;
                                acc.push(interp.handle(value));
                                Ok(acc)
                            })
                            .await?;
                        let func = interp.eval(env, car).await?;
                        func.apply(interp, env, arg_handles.iter().map(|h| h.value()).collect())
                            .await
                    }
                }
                HeapObject::Symbol(name) => {
                    let env = interp.to_env(env);
                    match env.borrow().lookup(interp, id) {
                        Some(value) => return Ok(EvalResult::Done(value)),
                        None => {
                            return Err(SchemeError::UnboundVariable(format!(
                                "Unbound symbol: {}",
                                name
                            )));
                        }
                    }
                }
                HeapObject::FreeSlot(_) => panic!("Request to evaluate FreeSlot at {}", id),
                _ => return Ok(EvalResult::Done(Value::Object(id))),
            }
        })
    }

    fn is_false(&self) -> bool {
        return *self == Keyword::False as usize;
    }

    fn write(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = *self;
        let heap = interp.heap.borrow();
        let obj = heap.get(id);
        match obj {
            HeapObject::Pair(car, cdr) => {
                let mut p = cdr.clone();
                write!(f, "(")?;
                car.write(interp, f)?;
                loop {
                    if let Some((cadr, cddr)) = interp.is_pair(p) {
                        write!(f, " ")?;
                        cadr.write(interp, f)?;
                        p = cddr;
                    } else if interp.is_nil(p) {
                        break;
                    } else {
                        write!(f, " . ")?;
                        p.write(interp, f)?;
                        break;
                    }
                }
                write!(f, ")")
            }
            HeapObject::Vector(v) => {
                write!(f, "#(")?;
                for (i, e) in v.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?; // Add a space before every element EXCEPT the first
                    }
                    e.write(interp, f)?;
                }
                write!(f, ")")
            }
            HeapObject::Symbol(s) => write!(f, "{}", s),
            HeapObject::String(s) => {
                write!(f, "{}", "\"")?;
                s.borrow().chars().try_fold((), |_, ch| match ch {
                    '\n' => write!(f, "\\n"),
                    '\t' => write!(f, "\\t"),
                    '\r' => write!(f, "\\r"),
                    '"' => write!(f, "\\\""),
                    _ => write!(f, "{}", ch),
                })?;
                write!(f, "{}", "\"")
            }
            HeapObject::Primitive(pr) => write!(f, "<{}>", pr.name),
            HeapObject::Closure(_) => write!(f, "<closure {}>", id),
            HeapObject::NaryClosure(_) => write!(f, "<n-closure {}>", id),
            HeapObject::InputPort(_) => write!(f, "<input-port {}>", id),
            HeapObject::OutputPort(_) => write!(f, "<output-port {}>", id),
            HeapObject::Env(_) => write!(f, "<env {id}>"),
            HeapObject::Foreign(foreign) => write!(f, "<foreign:{} {}>", foreign.type_name, id),
            HeapObject::FreeSlot(id) => panic!("Attempt to render free slot {}", id),
        }
    }

    fn display(&self, interp: &Scheme, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = *self;
        let heap = interp.heap.borrow();
        let obj = heap.get(id);
        match obj {
            HeapObject::Pair(car, cdr) => {
                let mut p = cdr.clone();
                write!(f, "(")?;
                car.display(interp, f)?;
                loop {
                    if let Some((cadr, cddr)) = interp.is_pair(p) {
                        write!(f, " ")?;
                        cadr.display(interp, f)?;
                        p = cddr;
                    } else if interp.is_nil(p) {
                        break;
                    } else {
                        write!(f, " . ")?;
                        p.display(interp, f)?;
                        break;
                    }
                }
                write!(f, ")")
            }
            HeapObject::Vector(v) => {
                write!(f, "#(")?;
                for (i, e) in v.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?; // Add a space before every element EXCEPT the first
                    }
                    e.display(interp, f)?;
                }
                write!(f, ")")
            }
            HeapObject::Symbol(s) => write!(f, "{}", s),
            HeapObject::String(s) => write!(f, "{}", s.borrow()),
            HeapObject::Primitive(pr) => write!(f, "<{}>", pr.name),
            HeapObject::Closure(_) => write!(f, "<closure {}>", id),
            HeapObject::NaryClosure(_) => write!(f, "<n-closure {}>", id),
            HeapObject::InputPort(_) => write!(f, "<input-port {}>", id),
            HeapObject::OutputPort(_) => write!(f, "<output-port {}>", id),
            HeapObject::Env(_) => write!(f, "<env {id}>"),
            HeapObject::Foreign(foreign) => write!(f, "<foreign:{} {}>", foreign.type_name, id),
            HeapObject::FreeSlot(id) => panic!("Attempt to render free slot {}", id),
        }
    }

    fn mark(&self, interp: &Scheme, marks: &mut MarkSet) {
        // If we've already been marked, no need to walk through again.
        let id = *self;
        if !marks.mark(id) {
            return;
        }
        let obj = {
            let heap = interp.heap.borrow();
            heap.get(id).clone()
        };
        match obj {
            HeapObject::Pair(car, cdr) => {
                car.mark(interp, marks);
                cdr.mark(interp, marks);
            }
            HeapObject::Vector(v) => {
                for item in v.borrow().iter() {
                    item.mark(interp, marks);
                }
            }
            HeapObject::Symbol(_) => {}
            HeapObject::String(_) => {}
            HeapObject::Primitive(_) => {}
            HeapObject::Closure(closure) | HeapObject::NaryClosure(closure) => {
                for id in closure.params {
                    id.mark(interp, marks);
                }
                for expr in closure.body {
                    expr.mark(interp, marks);
                }
                closure.tail.mark(interp, marks);
                closure.env.mark(interp, marks);
            }
            HeapObject::Env(env) => {
                env.borrow().mark(interp, marks);
            }
            HeapObject::InputPort(_) => {}
            HeapObject::OutputPort(_) => {}
            HeapObject::Foreign(_) => {}
            _ => {
                panic!("Request to mark a {}.", obj.type_name());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protection() {
        let heap = Heap::new(1024);
        {
            let _handle1 = heap.handle_id(1);
            assert!(heap.protected.borrow().get(&1).expect("msg") == &1);
            let _handle2 = heap.handle_id(1);
            assert!(heap.protected.borrow().get(&1).expect("msg") == &2);
        }
        assert!(heap.protected.borrow().is_empty());
    }
}
