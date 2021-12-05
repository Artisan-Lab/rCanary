#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables, unused_mut, dead_code))]

// This PoC reflects the leakage caused by lacking of a sound drop function for .
// Basically, these api are wrapperring 'ManuallyDrop' again and returns a raw pointer towards the heap data.
// Note that in this kind of pattern, the new instance with type ManaullyDop<T> will convert/cast into anther pointer-like type,
// all it wants to do is escaping from OBRM but still remaining a raw pointer to use the instance.
// Unfortunately, this ptr will never be recovered its ownership in the following data-flow that makes the leakage.

struct Foo<T> {
    ptr: *const T,
}

enum Bar<T> {
    Field1,
    Field2 { ptr: *const T },
}

fn main() {
    let b1 = Box::new("boxed");
    let b2 = Box::new(1);
    let foo = Foo { ptr: Box::into_raw(b1) };
    let bar = Bar::Field2 { ptr: Box::into_raw(b2) };
}