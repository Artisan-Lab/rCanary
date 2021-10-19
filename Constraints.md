


# RLC Analysis

## Type Collector
### Data Structure

1. MIR LOCAL
```rust
/// A MIR local.
/// This can be a binding declared by the user, a temporary inserted by the compiler, a function
/// argument, or the return place.
#[derive(Clone, Debug, TyEncodable, TyDecodable, HashStable, TypeFoldable)]
pub struct LocalDecl<'tcx> {
    /// Whether this is a mutable binding (i.e., `let x` or `let mut x`).
    ///
    /// Temporaries and the return place are always mutable.
    pub mutability: Mutability,

    // FIXME(matthewjasper) Don't store in this in `Body`
    pub local_info: Option<Box<LocalInfo<'tcx>>>,

    /// `true` if this is an internal local.
    ///
    /// These locals are not based on types in the source code and are only used
    /// for a few desugarings at the moment.
    ///
    /// The generator transformation will sanity check the locals which are live
    /// across a suspension point against the type components of the generator
    /// which type checking knows are live across a suspension point. We need to
    /// flag drop flags to avoid triggering this check as they are introduced
    /// after typeck.
    ///
    /// This should be sound because the drop flags are fully algebraic, and
    /// therefore don't affect the auto-trait or outlives properties of the
    /// generator.
    pub internal: bool,

    /// If this local is a temporary and `is_block_tail` is `Some`,
    /// then it is a temporary created for evaluation of some
    /// subexpression of some block's tail expression (with no
    /// intervening statement context).
    // FIXME(matthewjasper) Don't store in this in `Body`
    pub is_block_tail: Option<BlockTailInfo>,

    /// The type of this local.
    pub ty: Ty<'tcx>,

    /// If the user manually ascribed a type to this variable,
    /// e.g., via `let x: T`, then we carry that type here. The MIR
    /// borrow checker needs this information since it can affect
    /// region inference.
    // FIXME(matthewjasper) Don't store in this in `Body`
    pub user_ty: Option<Box<UserTypeProjections>>,

    /// The *syntactic* (i.e., not visibility) source scope the local is defined
    /// in. If the local was defined in a let-statement, this
    /// is *within* the let-statement, rather than outside
    /// of it.
    ///
    /// This is needed because the visibility source scope of locals within
    /// a let-statement is weird.
    ///
    /// The reason is that we want the local to be *within* the let-statement
    /// for lint purposes, but we want the local to be *after* the let-statement
    /// for names-in-scope purposes.
    ///
    /// That's it, if we have a let-statement like the one in this
    /// function:
    ///
    /// ```
    /// fn foo(x: &str) {
    ///     #[allow(unused_mut)]
    ///     let mut x: u32 = { // <- one unused mut
    ///         let mut y: u32 = x.parse().unwrap();
    ///         y + 2
    ///     };
    ///     drop(x);
    /// }
    /// ```
    ///
    /// Then, from a lint point of view, the declaration of `x: u32`
    /// (and `y: u32`) are within the `#[allow(unused_mut)]` scope - the
    /// lint scopes are the same as the AST/HIR nesting.
    ///
    /// However, from a name lookup point of view, the scopes look more like
    /// as if the let-statements were `match` expressions:
    ///
    /// ```
    /// fn foo(x: &str) {
    ///     match {
    ///         match x.parse().unwrap() {
    ///             y => y + 2
    ///         }
    ///     } {
    ///         x => drop(x)
    ///     };
    /// }
    /// ```
    ///
    /// We care about the name-lookup scopes for debuginfo - if the
    /// debuginfo instruction pointer is at the call to `x.parse()`, we
    /// want `x` to refer to `x: &str`, but if it is at the call to
    /// `drop(x)`, we want it to refer to `x: u32`.
    ///
    /// To allow both uses to work, we need to have more than a single scope
    /// for a local. We have the `source_info.scope` represent the "syntactic"
    /// lint scope (with a variable being under its let block) while the
    /// `var_debug_info.source_info.scope` represents the "local variable"
    /// scope (where the "rest" of a block is under all prior let-statements).
    ///
    /// The end result looks like this:
    ///
    /// ```text
    /// ROOT SCOPE
    ///  │{ argument x: &str }
    ///  │
    ///  │ │{ #[allow(unused_mut)] } // This is actually split into 2 scopes
    ///  │ │                         // in practice because I'm lazy.
    ///  │ │
    ///  │ │← x.source_info.scope
    ///  │ │← `x.parse().unwrap()`
    ///  │ │
    ///  │ │ │← y.source_info.scope
    ///  │ │
    ///  │ │ │{ let y: u32 }
    ///  │ │ │
    ///  │ │ │← y.var_debug_info.source_info.scope
    ///  │ │ │← `y + 2`
    ///  │
    ///  │ │{ let x: u32 }
    ///  │ │← x.var_debug_info.source_info.scope
    ///  │ │← `drop(x)` // This accesses `x: u32`.
    /// ```
    pub source_info: SourceInfo,
}
```
   

2. STATEMENT KIND
```rust
pub enum StatementKind<'tcx> {
   /// Write the RHS Rvalue to the LHS Place.
   Assign(Box<(Place<'tcx>, Rvalue<'tcx>)>),

   /// This represents all the reading that a pattern match may do
   /// (e.g., inspecting constants and discriminant values), and the
   /// kind of pattern it comes from. This is in order to adapt potential
   /// error messages to these specific patterns.
   ///
   /// Note that this also is emitted for regular `let` bindings to ensure that locals that are
   /// never accessed still get some sanity checks for, e.g., `let x: ! = ..;`
   FakeRead(Box<(FakeReadCause, Place<'tcx>)>),

   /// Write the discriminant for a variant to the enum Place.
   SetDiscriminant { place: Box<Place<'tcx>>, variant_index: VariantIdx },

   /// Start a live range for the storage of the local.
   StorageLive(Local),

   /// End the current live range for the storage of the local.
   StorageDead(Local),

   /// Executes a piece of inline Assembly. Stored in a Box to keep the size
   /// of `StatementKind` low.
   LlvmInlineAsm(Box<LlvmInlineAsm<'tcx>>),

   /// Retag references in the given place, ensuring they got fresh tags. This is
   /// part of the Stacked Borrows model. These statements are currently only interpreted
   /// by miri and only generated when "-Z mir-emit-retag" is passed.
   /// See <https://internals.rust-lang.org/t/stacked-borrows-an-aliasing-model-for-rust/8153/>
   /// for more details.
   Retag(RetagKind, Box<Place<'tcx>>),

   /// Encodes a user's type ascription. These need to be preserved
   /// intact so that NLL can respect them. For example:
   ///
   ///     let a: T = y;
   ///
   /// The effect of this annotation is to relate the type `T_y` of the place `y`
   /// to the user-given type `T`. The effect depends on the specified variance:
   ///
   /// - `Covariant` -- requires that `T_y <: T`
   /// - `Contravariant` -- requires that `T_y :> T`
   /// - `Invariant` -- requires that `T_y == T`
   /// - `Bivariant` -- no effect
   AscribeUserType(Box<(Place<'tcx>, UserTypeProjection)>, ty::Variance),

   /// Marks the start of a "coverage region", injected with '-Zinstrument-coverage'. A
   /// `Coverage` statement carries metadata about the coverage region, used to inject a coverage
   /// map into the binary. If `Coverage::kind` is a `Counter`, the statement also generates
   /// executable code, to increment a counter variable at runtime, each time the code region is
   /// executed.
   Coverage(Box<Coverage>),

   /// Denotes a call to the intrinsic function copy_overlapping, where `src_dst` denotes the
   /// memory being read from and written to(one field to save memory), and size
   /// indicates how many bytes are being copied over.
   CopyNonOverlapping(Box<CopyNonOverlapping<'tcx>>),

   /// No-op. Useful for deleting instructions without affecting statement indices.
   Nop,
}
```

3. TYS
```rust
#[allow(rustc::usage_of_ty_tykind)]
pub struct TyS<'tcx> {
    /// This field shouldn't be used directly and may be removed in the future.
    /// Use `TyS::kind()` instead.
    kind: TyKind<'tcx>,
    /// This field shouldn't be used directly and may be removed in the future.
    /// Use `TyS::flags()` instead.
    flags: TypeFlags,

    /// This is a kind of confusing thing: it stores the smallest
    /// binder such that
    ///
    /// (a) the binder itself captures nothing but
    /// (b) all the late-bound things within the type are captured
    ///     by some sub-binder.
    ///
    /// So, for a type without any late-bound things, like `u32`, this
    /// will be *innermost*, because that is the innermost binder that
    /// captures nothing. But for a type `&'D u32`, where `'D` is a
    /// late-bound region with De Bruijn index `D`, this would be `D + 1`
    /// -- the binder itself does not capture `D`, but `D` is captured
    /// by an inner binder.
    ///
    /// We call this concept an "exclusive" binder `D` because all
    /// De Bruijn indices within the type are contained within `0..D`
    /// (exclusive).
    outer_exclusive_binder: ty::DebruijnIndex,
}
```

4. TY KIND
```rust
/// Defines the kinds of types.
///
/// N.B., if you change this, you'll probably want to change the corresponding
/// AST structure in `rustc_ast/src/ast.rs` as well.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, TyEncodable, TyDecodable, Debug)]
#[derive(HashStable)]
#[rustc_diagnostic_item = "TyKind"]
pub enum TyKind<'tcx> {
    /// The primitive boolean type. Written as `bool`.
    Bool,

    /// The primitive character type; holds a Unicode scalar value
    /// (a non-surrogate code point). Written as `char`.
    Char,

    /// A primitive signed integer type. For example, `i32`.
    Int(ty::IntTy),

    /// A primitive unsigned integer type. For example, `u32`.
    Uint(ty::UintTy),

    /// A primitive floating-point type. For example, `f64`.
    Float(ty::FloatTy),

    /// Algebraic data types (ADT). For example: structures, enumerations and unions.
    ///
    /// InternalSubsts here, possibly against intuition, *may* contain `Param`s.
    /// That is, even after substitution it is possible that there are type
    /// variables. This happens when the `Adt` corresponds to an ADT
    /// definition and not a concrete use of it.
    Adt(&'tcx AdtDef, SubstsRef<'tcx>),

    /// An unsized FFI type that is opaque to Rust. Written as `extern type T`.
    Foreign(DefId),

    /// The pointee of a string slice. Written as `str`.
    Str,

    /// An array with the given length. Written as `[T; n]`.
    Array(Ty<'tcx>, &'tcx ty::Const<'tcx>),

    /// The pointee of an array slice. Written as `[T]`.
    Slice(Ty<'tcx>),

    /// A raw pointer. Written as `*mut T` or `*const T`
    RawPtr(TypeAndMut<'tcx>),

    /// A reference; a pointer with an associated lifetime. Written as
    /// `&'a mut T` or `&'a T`.
    Ref(Region<'tcx>, Ty<'tcx>, hir::Mutability),

    /// The anonymous type of a function declaration/definition. Each
    /// function has a unique type, which is output (for a function
    /// named `foo` returning an `i32`) as `fn() -> i32 {foo}`.
    ///
    /// For example the type of `bar` here:
    ///
    /// ```rust
    /// fn foo() -> i32 { 1 }
    /// let bar = foo; // bar: fn() -> i32 {foo}
    /// ```
    FnDef(DefId, SubstsRef<'tcx>),

    /// A pointer to a function. Written as `fn() -> i32`.
    ///
    /// For example the type of `bar` here:
    ///
    /// ```rust
    /// fn foo() -> i32 { 1 }
    /// let bar: fn() -> i32 = foo;
    /// ```
    FnPtr(PolyFnSig<'tcx>),

    /// A trait object. Written as `dyn for<'b> Trait<'b, Assoc = u32> + Send + 'a`.
    Dynamic(&'tcx List<Binder<'tcx, ExistentialPredicate<'tcx>>>, ty::Region<'tcx>),

    /// The anonymous type of a closure. Used to represent the type of
    /// `|a| a`.
    Closure(DefId, SubstsRef<'tcx>),

    /// The anonymous type of a generator. Used to represent the type of
    /// `|a| yield a`.
    Generator(DefId, SubstsRef<'tcx>, hir::Movability),

    /// A type representing the types stored inside a generator.
    /// This should only appear in GeneratorInteriors.
    GeneratorWitness(Binder<'tcx, &'tcx List<Ty<'tcx>>>),

    /// The never type `!`.
    Never,

    /// A tuple type. For example, `(i32, bool)`.
    /// Use `TyS::tuple_fields` to iterate over the field types.
    Tuple(SubstsRef<'tcx>),

    /// The projection of an associated type. For example,
    /// `<T as Trait<..>>::N`.
    Projection(ProjectionTy<'tcx>),

    /// Opaque (`impl Trait`) type found in a return type.
    /// The `DefId` comes either from
    /// * the `impl Trait` ast::Ty node,
    /// * or the `type Foo = impl Trait` declaration
    /// The substitutions are for the generics of the function in question.
    /// After typeck, the concrete type can be found in the `types` map.
    Opaque(DefId, SubstsRef<'tcx>),

    /// A type parameter; for example, `T` in `fn f<T>(x: T) {}`.
    Param(ParamTy),

    /// Bound type variable, used only when preparing a trait query.
    Bound(ty::DebruijnIndex, BoundTy),

    /// A placeholder type - universally quantified higher-ranked type.
    Placeholder(ty::PlaceholderType),

    /// A type variable used during type checking.
    Infer(InferTy),

    /// A placeholder for a type which could not be computed; this is
    /// propagated to avoid useless error messages.
    Error(DelaySpanBugEmitted),
}
```

### Intra-procedural Analysis

```rust
// The ownership of RLC
// all types that not having a heap chunck will be ignored in rlc
```

#### ADT Types Constraints (ATC)

Give three structs: `Foo` `Bar` `Baz`, and all of them contain a pointer (include reference/raw pointer/slice/smart pointer) pointing to one place. For now, we assume that we do not know which kind of data (heap/stack) they are pointing at.

```rust
struct Foo<T> {
   ptr_mut: *mut T,
   ptr_immut: *const T,
}

struct Bar<T> {
   ptr: *mut T,
   _marker: PhantomData<T>,
}

struct Baz<T> {
   ptr: Vec<T>,
}
```

```rust
struct Slice<'a, T> {
    start: *const T,
    end: *const T,
}

struct Slice<'a, T: 'a> {
   start: *const T,
   end: *const T,
   phantom: PhantomData<&'a T>,
}
```

Now we give 3 heap-constraints for ADT types. For an ADT type:
1. if one field is `[T]`  `&mut T` `&T` => _**NOT OWNED**_
2. if one field is `*mut T` `*const T` **but** not associated with `PhantomData<T>` => _**NOT OWNED**_
3. if one field is `*mut T` `*const T` **and** associated with `PhantomData<T>` => _**OWNED**_: calculate sum of _**OWNED**_ pointer in this struct
4. if one field is `PhantomData<T>` alone => search for other structs having `<T>` with raw pointer types => **depth:2** (eg. `NonNull<T>`)
5. if one field is _**OWNED**_ type, marked the whole type and _**OWNED**_ field
   1. if `COUNT` is `1` => the owner is whole **struct** but not `*mut T` etc..
   2. if `COUNT` > `1` => mark each _**OWNED**_ filed in this struct
6. **collections/boxed** types are regarding as _**OWNED**_ types to avoid analysis: `Box<T>` `String` `Rc<T>` `Vec<T>` etc.. (std::collections)
7. if it is an anonymous-struct or a ~~tuple struct~~ => alike before
8. if one variant is `[]` `*mut T` `*const T` `&mut T` `&T` => _**NOT OWNED**_
9. if one variant is associated with a struct => _**Depend on the result of this struct**_

How we perform ATC:
1. Identify all structs and enums in library and binary crates, extract all types `Ty::ty` and cache them into one set. -> to json
    * Ideally, we will traverse all `Defid` with `tcx.optimized_mir()` and collect all types into this set
    * Through `tcx.optimized_mir()` we perform a easy inter-procedural operation to its callee and collect these types that defined in dependencies
    * Second choice is add a `GLOBAL_COLLECTOR` query to rustc => makes rlc more complicated
    * Another choice is using `librustdoc` => the `Ty::ty` may not complete
2. Construct a dependency graph of all types through this set, and calculate the topo-order of all types.
3. Use topo-order to analysis the constructor and destructor in llvm, the result is the `vector<bool>` to identify where one instance of this type will host actual heap.