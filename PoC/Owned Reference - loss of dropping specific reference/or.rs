#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code))]

use std::mem::ManuallyDrop;

enum E {
    A(i32),
    B,
}

fn main() {
    let b = Box::new("boxed");
    let r = ManuallyDrop::new(b).as_ref();
    let x = E::A(1);
    let y = E::B;
}