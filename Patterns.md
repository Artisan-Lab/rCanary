# CVE for RLC Analysis

* flaco pr#12 (**Proxy-Type: loss of OWNED-FIELD DROP -- impl Drop**)

[https://github.com/milesgranger/flaco/pull/12/commits
](https://github.com/milesgranger/flaco/pull/12/commits)

```rust
#[derive(Debug)]
#[repr(C)]
pub enum Data {
    Bytes(BytesPtr),
    Boolean(bool),
    Decimal(f64), // TODO: support lossless decimal/numeric type handling
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(StringPtr),
    Null,
}

#[derive(Debug)]
#[repr(C)]
pub struct BytesPtr {
    pub ptr: *mut u8,
    pub len: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct StringPtr {
    pub ptr: *mut c_char,
    pub len: u32,
}
```

```rust
// leak ver.
fn from(val: Option<Vec<u8>>) -> Self {
        match val {
            Some(v) => {
                let ptr = v.as_ptr() as _;
                let len = v.len() as u32;
                mem::forget(v);
// @@ -173,15 +174,28 @@ impl From<Option<String>> for Data {
        match val {
            Some(string) => {
                let cstring = ffi::CString::new(string).unwrap();
                let ptr = cstring.as_ptr();
                mem::forget(cstring);
                Data::String(ptr)
            }
            None => Data::Null,
        }
    }
}
```

```rust
// fixed ver
impl From<Option<Vec<u8>>> for Data {
    fn from(val: Option<Vec<u8>>) -> Self {
        match val {
            Some(mut v) => {
                v.shrink_to_fit();
                let ptr = v.as_ptr() as _;
                let len = v.len() as u32;
                mem::forget(v);
// @@ -173,15 +174,28 @@ impl From<Option<String>> for Data {
        match val {
            Some(string) => {
                let cstring = ffi::CString::new(string).unwrap();
                Data::String(cstring.into_raw())
            }
            None => Data::Null,
        } 
    }
}

 // special drop for any variants containing pointers
impl Drop for Data {
    fn drop(&mut self) {
        match self {
            Data::String(ptr) => {
                let _ = unsafe { CString::from_raw(*ptr as _) };
            }
            Data::Bytes(ptr) => {
                let _ = unsafe { Vec::from_raw_parts(ptr.ptr, ptr.len as usize, ptr.len as usize) };
            }
            _ => (),
        }
    }
}
```

* ringbuffer pr#51 (**Proxy-Type: loss of Proxy-Type DROP -- DRAIN / drop(_)**)

[https://github.com/NULLx76/ringbuffer/pull/51
](https://github.com/NULLx76/ringbuffer/pull/51)


```rust
// std:drain::Drain
/// A draining iterator for `Vec<T>`.
///
/// This `struct` is created by [`Vec::drain`].
/// See its documentation for more.
///
/// # Example
///
/// ```
/// let mut v = vec![0, 1, 2];
/// let iter: std::vec::Drain<_> = v.drain(..);
/// ```
#[stable(feature = "drain", since = "1.6.0")]
pub struct Drain<
    'a,
    T: 'a,
    #[unstable(feature = "allocator_api", issue = "32838")] A: Allocator + 'a = Global,
> {
    /// Index of tail to preserve
    pub(super) tail_start: usize,
    /// Length of tail
    pub(super) tail_len: usize,
    /// Current remaining range to remove
    pub(super) iter: slice::Iter<'a, T>,
    pub(super) vec: NonNull<Vec<T, A>>,
}

#[stable(feature = "drain", since = "1.6.0")]
impl<T, A: Allocator> Drop for Drain<'_, T, A> {
    fn drop(&mut self) {
        /// Continues dropping the remaining elements in the `Drain`, then moves back the
        /// un-`Drain`ed elements to restore the original `Vec`.
        struct DropGuard<'r, 'a, T, A: Allocator>(&'r mut Drain<'a, T, A>);

        impl<'r, 'a, T, A: Allocator> Drop for DropGuard<'r, 'a, T, A> {
            fn drop(&mut self) {
                // Continue the same loop we have below. If the loop already finished, this does
                // nothing.
                self.0.for_each(drop);

                if self.0.tail_len > 0 {
                    unsafe {
                        let source_vec = self.0.vec.as_mut();
                        // memmove back untouched tail, update to new length
                        let start = source_vec.len();
                        let tail = self.0.tail_start;
                        if tail != start {
                            let src = source_vec.as_ptr().add(tail);
                            let dst = source_vec.as_mut_ptr().add(start);
                            ptr::copy(src, dst, self.0.tail_len);
                        }
                        source_vec.set_len(start + self.0.tail_len);
                    }
                }
            }
        }

        // exhaust self first
        while let Some(item) = self.next() {
            let guard = DropGuard(self);
            drop(item);
            mem::forget(guard);
        }

        // Drop a `DropGuard` to move back the non-drained tail of `self`.
        DropGuard(self);
    }
}

/// Creates a draining iterator that removes the specified range in the vector
/// and yields the removed items.
///
/// When the iterator **is** dropped, all elements in the range are removed
/// from the vector, even if the iterator was not fully consumed. If the
/// iterator **is not** dropped (with [`mem::forget`] for example), it is
/// unspecified how many elements are removed.
///
/// # Panics
///
/// Panics if the starting point is greater than the end point or if
/// the end point is greater than the length of the vector.
///
/// # Examples
///
/// ```
/// let mut v = vec![1, 2, 3];
/// let u: Vec<_> = v.drain(1..).collect();
/// assert_eq!(v, &[1]);
/// assert_eq!(u, &[2, 3]);
///
/// // A full range clears the vector
/// v.drain(..);
/// assert_eq!(v, &[]);
/// ```
#[stable(feature = "drain", since = "1.6.0")]
pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>
    where
        R: RangeBounds<usize>,
{
    // Memory safety
    //
    // When the Drain is first created, it shortens the length of
    // the source vector to make sure no uninitialized or moved-from elements
    // are accessible at all if the Drain's destructor never gets to run.
    //
    // Drain will ptr::read out the values to remove.
    // When finished, remaining tail of the vec is copied back to cover
    // the hole, and the vector length is restored to the new length.
    //
    let len = self.len();
    let Range { start, end } = slice::range(range, ..len);

    unsafe {
        // set self.vec length's to start, to be safe in case Drain is leaked
        self.set_len(start);
        // Use the borrow in the IterMut to indicate borrowing behavior of the
        // whole Drain iterator (like &mut T).
        let range_slice = slice::from_raw_parts_mut(self.as_mut_ptr().add(start), end - start);
        Drain {
            tail_start: end,
            tail_len: len - end,
            iter: range_slice.iter(),
            vec: NonNull::from(self),
        }
    }
}

```

```rust
/// Defines behaviour for ringbuffers which allow for reading from the start of them (as a queue).
/// For arbitrary buffer access however, [`RingBufferExt`] is necessary.
pub trait RingBufferRead<T>: RingBuffer<T> {
    /// Returns an iterator over the elements in the ringbuffer,
    /// dequeueing elements as they are iterated over.
    ///
    /// ```
    /// use ringbuffer::{AllocRingBuffer, RingBufferWrite, RingBufferRead, RingBuffer};
    ///
    /// let mut rb = AllocRingBuffer::with_capacity(16);
    /// for i in 0..8 {
    ///     rb.push(i);
    /// }
    ///
    /// assert_eq!(rb.len(), 8);
    ///
    /// for i in rb.drain() {
    ///     // prints the numbers 0 through 8
    ///     println!("{}", i);
    /// }
    ///
    /// // No elements remain
    /// assert_eq!(rb.len(), 0);
    ///
    /// ```
    fn drain(&mut self) -> RingBufferDrainingIterator<T, Self> {
        RingBufferDrainingIterator::new(self)
    }
}

/// Defines behaviour for ringbuffers which allow them to be used as a general purpose buffer.
/// With this trait, arbitrary access of elements in the buffer is possible.
pub trait RingBufferExt<T>:
RingBuffer<T>
+ RingBufferRead<T>
+ RingBufferWrite<T>
+ Index<isize, Output = T>
+ IndexMut<isize>
+ FromIterator<T>
{
    /// Empties the buffer entirely. Sets the length to 0 but keeps the capacity allocated.
    fn clear(&mut self);
}
```

```rust
// leak ver
macro_rules! impl_ringbuffer_read {
    # [inline]
    fn clear( & mut self) {
        self.$readptr = 0;
        self.$writeptr = 0;
    }
    ...
}
```

```rust
// fixed ver
macro_rules! impl_ringbuffer_read {
    # [inline]
    fn fixed( & mut self) {
        for i in self.drain() {
            drop(i);
        }
        self.$readptr = 0;
        self.$writeptr = 0;
    }
    ...
}
```

* pprof-rs pr#84 (**Improper Lifetime: static-lifetime OWNED items**)

[https://github.com/tikv/pprof-rs/pull/84](https://github.com/tikv/pprof-rs/pull/84)

```rust
// leak ver
pub struct Bucket<T: 'static> {
    pub length: usize,
    entries: &'static mut [Entry<T>; BUCKETS_ASSOCIATIVITY],
}

pub struct StackHashCounter<T: Hash + Eq + 'static> {
    buckets: &'static mut [Bucket<T>; BUCKETS],
}

pub struct TempFdArray<T: 'static> {
    file: NamedTempFile,
    buffer: &'static mut [T; BUFFER_LENGTH],
    buffer_index: usize,
}

impl<T: Eq> Default for Bucket<T> {
    fn default() -> Bucket<T> {
        let entries = Box::new(unsafe { std::mem::MaybeUninit::uninit().assume_init() });
        Self {
            length: 0,
            entries: Box::leak(entries),
        }
    }
}
```

```rust
// fixed ver
pub struct Bucket<T: 'static> {
    pub length: usize,
    entries: Box<[Entry<T>; BUCKETS_ASSOCIATIVITY]>,
}

pub struct HashCounter<T: Hash + Eq + 'static> {
    buckets: Box<[Bucket<T>; BUCKETS]>,
}

pub struct TempFdArray<T: 'static> {
    file: NamedTempFile,
    buffer: Box<[T; BUFFER_LENGTH]>,
    buffer_index: usize,
}

impl<T: Eq> Default for Bucket<T> {
    fn default() -> Bucket<T> {
        let entries = Box::new(unsafe { std::mem::MaybeUninit::uninit().assume_init() });
        Self { length: 0, entries }
    }
}
```

* rust-rocksdb pr#658 (**OwnedPointer: loss of dropping an Owned-Raw-Pointer**)

[https://github.com/tikv/rust-rocksdb/pull/658
](https://github.com/tikv/rust-rocksdb/pull/658)

```rust
impl WriteBatch {
    // leak ver
    pub fn iterate<F>(&self, cfs: &[&str], mut iterator_fn: F)
        where
            F: FnMut(&str, DBValueType, &[u8], Option<&[u8]>),
    {
        unsafe {
            let mut cb: &mut dyn FnMut(&str, DBValueType, &[u8], Option<&[u8]>) = &mut iterator_fn;
            let cb_ptr = &mut cb;
            let cb_proxy = Box::new(WriteBatchCallback {
                cfs,
                cb_ptr: cb_ptr as *mut _ as *mut c_void,
            });
            let state = Box::into_raw(cb_proxy) as *mut c_void;
            crocksdb_ffi::crocksdb_writebatch_iterate_cf(
                self.inner,
                state,
                put_fn,
                put_cf_fn,
                delete_fn,
                delete_cf_fn,
            );
            // Let rust free the memory
            let _ = *(state as *const WriteBatchCallback);
        }
    }
}
```

```rust
impl WriteBatch {
    //fixed ver
    pub fn iterate<F>(&self, cfs: &[&str], mut iterator_fn: F)
        where
            F: FnMut(&str, DBValueType, &[u8], Option<&[u8]>),
    {
        unsafe {
            let mut cb: &mut dyn FnMut(&str, DBValueType, &[u8], Option<&[u8]>) = &mut iterator_fn;
            let cb_ptr = &mut cb;
            let cb_proxy = Box::new(WriteBatchCallback {
                cfs,
                cb_ptr: cb_ptr as *mut _ as *mut c_void,
            });
            let state = Box::into_raw(cb_proxy) as *mut c_void;
            crocksdb_ffi::crocksdb_writebatch_iterate_cf(
                self.inner,
                state,
                put_fn,
                put_cf_fn,
                delete_fn,
                delete_cf_fn,
            );
            // Let rust free the memory
            let _ = Box::from_raw(state as *mut WriteBatchCallback);
        }
    }
}
```

* rowan pr#112 (**OwnedInstance: loss of dropping an Owned-ManuallyDrop-Instance**)

[https://github.com/rust-analyzer/rowan/pull/112](https://github.com/rust-analyzer/rowan/pull/112)

```rust
impl NodeData {
    #[inline]
    fn new(
        parent: Option<SyntaxNode>,
        index: u32,
        offset: TextSize,
        green: Green,
        mutable: bool,
    ) -> ptr::NonNull<NodeData> {
        let parent = ManuallyDrop::new(parent);
        let res = NodeData {
            _c: Count::new(),
            rc: Cell::new(1),
            parent: {
                let parent = ManuallyDrop::new(parent);
                Cell::new(parent.as_ref().map(|it| it.ptr))
            },
            index: Cell::new(index),
            green,

            mutable,
            offset,
            first: Cell::new(ptr::null()),
            next: Cell::new(ptr::null()),
            prev: Cell::new(ptr::null()),
        };
        unsafe {
            let mut res = Box::into_raw(Box::new(res));
            if mutable {
                if let Err(node) = sll::init((*res).parent().map(|it| &it.first), &*res) {
                    if cfg!(debug_assertions) {
                        assert_eq!((*node).index(), (*res).index());
                        match ((*node).green(), (*res).green()) {
                            (NodeOrToken::Node(lhs), NodeOrToken::Node(rhs)) => {
                                assert!(ptr::eq(lhs, rhs))
                            }
                            (NodeOrToken::Token(lhs), NodeOrToken::Token(rhs)) => {
                                assert!(ptr::eq(lhs, rhs))
                            }
                            it => {
                                panic!("node/token confusion: {:?}", it)
                            }
                        }
                    }

                    Box::from_raw(res);
                    res = node as *mut _;
                    (*res).inc_rc();
                }
            }
            ptr::NonNull::new_unchecked(res)
        }
    }
}
```

```rust
impl NodeData {
    #[inline]
    fn new(
        parent: Option<SyntaxNode>,
        index: u32,
        offset: TextSize,
        green: Green,
        mutable: bool,
    ) -> ptr::NonNull<NodeData> {
        let parent = ManuallyDrop::new(parent);
        let res = NodeData {
            _c: Count::new(),
            rc: Cell::new(1),
            parent: Cell::new(parent.as_ref().map(|it| it.ptr)),
            index: Cell::new(index),
            green,

            mutable,
            offset,
            first: Cell::new(ptr::null()),
            next: Cell::new(ptr::null()),
            prev: Cell::new(ptr::null()),
        };
        unsafe {
            let mut res = Box::into_raw(Box::new(res));
            if mutable {
                if let Err(node) = sll::init((*res).parent().map(|it| &it.first), &*res) {
                    if cfg!(debug_assertions) {
                        assert_eq!((*node).index(), (*res).index());
                        match ((*node).green(), (*res).green()) {
                            (NodeOrToken::Node(lhs), NodeOrToken::Node(rhs)) => {
                                assert!(ptr::eq(lhs, rhs))
                            }
                            (NodeOrToken::Token(lhs), NodeOrToken::Token(rhs)) => {
                                assert!(ptr::eq(lhs, rhs))
                            }
                            it => {
                                panic!("node/token confusion: {:?}", it)
                            }
                        }
                    }

                    Box::from_raw(res);
                    ManuallyDrop::into_inner(parent);
                    res = node as *mut _;
                    (*res).inc_rc();
                }
            }
            ptr::NonNull::new_unchecked(res)
        }
    }
}
```