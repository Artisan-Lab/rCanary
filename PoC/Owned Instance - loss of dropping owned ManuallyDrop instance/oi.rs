#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code))]

use std::mem::ManuallyDrop;

// This PoC reflects the leakage caused by 'ManuallyDrop' and it is also the most common case in rust (including unsafe).
// If the instance wrapperred by 'MaunallyDrop' is allocated in heap, it will never be deallocated by OBRM system.
// Note that in this kind of pattern, the new instance with type ManaullyDop<T> will not convert/cast into anther pointer-like type,
// all it wants to do is marking one variable as un_dropping to escape from OBRM.

fn main() {
    let b = Box::new("boxed");
    let b1 = ManuallyDrop::new(b);
}