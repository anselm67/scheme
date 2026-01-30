use std::collections::HashMap;

use crate::types::{GcId, Value};


pub struct Env {
    pub frames: Vec<HashMap<GcId, Value>>,
}

impl Env {
    
    pub fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    pub fn define(&mut self, key: GcId, value: Value) {
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(key, value);
        }
    }

    pub fn lookup(&self, key: GcId) -> Option<Value> {
        for frame in self.frames.iter().rev() {
            if let Some(value) = frame.get(&key) {
                return Some(*value);
            }
        }
        None
    }
}