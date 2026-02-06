pub mod types;
pub mod heap;
pub mod interp;
pub mod env;
pub mod parser;
pub mod macros;
#[cfg(test)]
mod tests {
    mod test_heap;
    mod test_eval;
    mod test_interp;
    mod test_parser;
}