#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code))]

// This PoC reflects the leakage caused by the specific api that producing one leaked pointer.
// Basically, these api are wrapperring 'ManuallyDrop' again and returns a raw pointer towards the heap data.
// Note that in this kind of pattern, the new instance with type ManaullyDop<T> will convert/cast into anther pointer-like type,
// all it wants to do is escaping from OBRM but still remaining a raw pointer to use the instance.
// Unfortunately, this ptr will never be recovered its ownership in the following data-flow that makes the leakage.

fn main() {
    let b = Box::new("boxed");
    let ptr = Box::into_raw(b);
}