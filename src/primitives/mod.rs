mod char;
mod eval;
mod list;
mod number;
mod port;
mod string;
mod system;
mod vector;
use crate::interp::Interp;

pub fn register_all(interp: &Interp) {
    eval::register(interp);
    system::register(interp);
    number::register(interp);
    string::register(interp);
    char::register(interp);
    list::register(interp);
    vector::register(interp);
    port::register(interp);
}
