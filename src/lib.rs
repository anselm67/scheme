pub mod env;
pub mod heap;
pub mod interp;
pub mod parser;
pub mod repl;
pub mod types;
#[macro_use]
pub mod macros;
pub mod markset;
mod primitives;
#[cfg(test)]
mod tests {
    mod test_eval;
    mod test_heap;
    mod test_interp;
    mod test_parser;
    mod test_scheme;
}
