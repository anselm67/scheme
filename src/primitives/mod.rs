mod char;
mod eval;
mod list;
mod number;
mod port;
mod string;
mod system;
mod vector;
mod async_primitives;
use crate::interp::Scheme;

pub fn register_all(interp: &Scheme) {
    eval::register(interp);
    system::register(interp);
    number::register(interp);
    string::register(interp);
    char::register(interp);
    list::register(interp);
    vector::register(interp);
    port::register(interp);
    async_primitives::register(interp);
}
