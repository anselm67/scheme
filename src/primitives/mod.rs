mod char;
mod list;
mod number;
mod port;
mod string;
mod vector;

use crate::interp::Interp;

pub fn register_all(interp: &Interp) {
    number::register(interp);
    string::register(interp);
    char::register(interp);
    list::register(interp);
    vector::register(interp);
    port::register(interp);
}
