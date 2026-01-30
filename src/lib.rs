pub mod types;
pub mod heap;
pub mod interp;
pub mod env;

#[cfg(test)]
mod test {
    mod test_heap;
    mod test_eval;
}