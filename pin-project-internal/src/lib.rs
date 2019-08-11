//! An interal crate to support pin_project - **do not use directly**

#![recursion_limit = "256"]
#![doc(html_root_url = "https://docs.rs/pin-project-internal/0.4.0-alpha.1")]
#![doc(test(attr(deny(warnings), allow(dead_code, unused_assignments, unused_variables))))]
#![warn(unsafe_code)]
#![warn(rust_2018_idioms, unreachable_pub)]
#![warn(single_use_lifetimes)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::use_self)]

extern crate proc_macro;

#[macro_use]
mod utils;

mod pin_project;
mod pinned_drop;
#[cfg(feature = "project_attr")]
mod project;

use proc_macro::TokenStream;
use utils::Nothing;

// TODO: Move this doc into pin-project crate when https://github.com/rust-lang/rust/pull/62855 merged.
/// An attribute that creates a projection struct covering all the fields.
///
/// This attribute creates a projection struct according to the following rules:
///
/// - For the field that uses `#[pin]` attribute, makes the pinned reference to
/// the field.
/// - For the other fields, makes the unpinned reference to the field.
///
/// ## Safety
///
/// This attribute is completely safe. In the absence of other `unsafe` code *that you write*,
/// it is impossible to cause undefined behavior with this attribute.
///
/// This is accomplished by enforcing the four requirements for pin projection
/// stated in [the Rust documentation](https://doc.rust-lang.org/beta/std/pin/index.html#projections-and-structural-pinning):
///
/// 1. The struct must only be Unpin if all the structural fields are Unpin
///
///	   To enforce this, this attribute will automatically generate an `Unpin` implementation
///    for you, which will require that all structurally pinned fields be `Unpin`
///    If you wish to provide an manual `Unpin` impl, you can do so via the
///    `UnsafeUnpin` argument.
///
/// 2. The destructor of the struct must not move structural fields out of its argument.
///
///    To enforce this, this attribute will automatically generate a `Drop` impl.
///    If you wish to provide a custom `Drop` impl, you can annotate a function
///    with `#[pinned_drop]`. This function takes a pinned version of your struct -
///    that is, `Pin<&mut MyStruct>` where `MyStruct` is the type of your struct.
///
///    You can call `project()` on this type as usual, along with any other
///    methods you have defined. Because your code is never provided with
///    a `&mut MyStruct`, it is impossible to move out of pin-projectable
///    fields in safe code in your destructor.
///
/// 3. You must make sure that you uphold the Drop guarantee: once your struct is pinned,
///    the memory that contains the content is not overwritten or deallocated without calling the content's destructors
///
///    Safe code doesn't need to worry about this - the only wait to violate this requirement
///    is to manually deallocate memory (which is `unsafe`), or to overwite a field with something else.
///    Becauese your custom destructor takes `Pin<&mut MyStruct`, it's impossible to obtain
///    a mutable reference to a pin-projected field in safe code.
///
/// 4. You must not offer any other operations that could lead to data being moved out of the structural fields when your type is pinned.
///
///    As with requirement 3, it is impossible for safe code to violate this. This crate ensures that safe code can never
///    obtain a mutable reference to `#[pin]` fields, which prevents you from ever moving out of them in safe code.
///
/// Pin projections are also incompatible with `#[repr(packed)]` structs. Attempting to use this attribute
/// on a `#[repr(packed)]` struct results in a compile-time error.
///
///
/// ## Examples
///
/// Using `#[pin_project]` will automatically create the appropriate
/// conditional [`Unpin`] implementation:
///
/// ```rust
/// use pin_project::pin_project;
/// use std::pin::Pin;
///
/// #[pin_project]
/// struct Foo<T, U> {
///     #[pin]
///     future: T,
///     field: U,
/// }
///
/// impl<T, U> Foo<T, U> {
///     fn baz(self: Pin<&mut Self>) {
///         let this = self.project();
///         let _: Pin<&mut T> = this.future; // Pinned reference to the field
///         let _: &mut U = this.field; // Normal reference to the field
///     }
/// }
/// ```
///
/// If you want to implement [`Unpin`] manually, you must use the `UnsafeUnpin`
/// argument to `#[pin_project]`.
///
/// ```rust
/// use pin_project::{pin_project, UnsafeUnpin};
/// use std::pin::Pin;
///
/// #[pin_project(UnsafeUnpin)]
/// struct Foo<T, U> {
///     #[pin]
///     future: T,
///     field: U,
/// }
///
/// impl<T, U> Foo<T, U> {
///     fn baz(self: Pin<&mut Self>) {
///         let this = self.project();
///         let _: Pin<&mut T> = this.future; // Pinned reference to the field
///         let _: &mut U = this.field; // Normal reference to the field
///     }
/// }
///
/// unsafe impl<T: Unpin, U> UnsafeUnpin for Foo<T, U> {} // Conditional Unpin impl
/// ```
///
/// Note the usage of the unsafe [`UnsafeUnpin`] trait, instead of the usual
/// [`Unpin`] trait. [`UnsafeUnpin`] behaves exactly like [`Unpin`], except that is
/// unsafe to implement. This unsafety comes from the fact that pin projections
/// are being used. If you implement [`UnsafeUnpin`], you must ensure that it is
/// only implemented when all pin-projected fields implement [`Unpin`].
///
/// Note that borrowing the field where `#[pin]` attribute is used multiple
/// times requires using [`.as_mut()`][`Pin::as_mut`] to avoid
/// consuming the `Pin`.
///
/// ### `#[pinned_drop]`
///
/// In order to correctly implement pin projections, a type's `Drop` impl must
/// not move out of any stucturally pinned fields. Unfortunately, [`Drop::drop`]
/// takes `&mut Self`, not `Pin<&mut Self>`.
///
/// To ensure that this requirement is upheld, the `pin_project` attribute will
/// provide a `Drop` impl for you. This `Drop` impl will delegate to a function
/// annotated with `#[pinned_drop]` if you use the `PinnedDrop` argument to
/// `#[pin_project]`. This function acts just like a normal [`drop`] impl, except
/// for the fact that it takes `Pin<&mut Self>`. In particular, it will never be
/// called more than once, just like [`Drop::drop`].
///
/// For example:
///
/// ```rust
/// use pin_project::{pin_project, pinned_drop};
/// use std::fmt::Debug;
/// use std::pin::Pin;
///
/// #[pin_project(PinnedDrop)]
/// pub struct Foo<T: Debug, U: Debug> {
///     #[pin] pinned_field: T,
///     unpin_field: U
/// }
///
/// #[pinned_drop]
/// fn my_drop_fn<T: Debug, U: Debug>(foo: Pin<&mut Foo<T, U>>) {
///     let foo = foo.project();
///     println!("Dropping pinned field: {:?}", foo.pinned_field);
///     println!("Dropping unpin field: {:?}", foo.unpin_field);
/// }
///
/// fn main() {
///     Foo { pinned_field: true, unpin_field: 40 };
/// }
/// ```
///
/// See also [`pinned_drop`] attribute.
///
/// ## Supported Items
///
/// The current pin-project supports the following types of items.
///
/// ### Structs (structs with named fields):
///
/// ```rust
/// use pin_project::pin_project;
/// use std::pin::Pin;
///
/// #[pin_project]
/// struct Foo<T, U> {
///     #[pin]
///     future: T,
///     field: U,
/// }
///
/// impl<T, U> Foo<T, U> {
///     fn baz(self: Pin<&mut Self>) {
///         let this = self.project();
///         let _: Pin<&mut T> = this.future;
///         let _: &mut U = this.field;
///     }
/// }
/// ```
///
/// ### Tuple structs (structs with unnamed fields):
///
/// ```rust
/// use pin_project::pin_project;
/// use std::pin::Pin;
///
/// #[pin_project]
/// struct Foo<T, U>(#[pin] T, U);
///
/// impl<T, U> Foo<T, U> {
///     fn baz(self: Pin<&mut Self>) {
///         let this = self.project();
///         let _: Pin<&mut T> = this.0;
///         let _: &mut U = this.1;
///     }
/// }
/// ```
///
/// Structs without fields (unit-like struct and zero fields struct) are not
/// supported.
///
/// ### Enums
///
/// `pin_project` also supports enums, but to use it ergonomically, you need
/// to use the [`project`] attribute.
///
/// *This attribute is only available if pin-project is built
/// with the `"project_attr"` feature.*
///
/// The attribute at the expression position is not stable, so you need to use
/// a dummy `#[project]` attribute for the function.
///
/// ```rust
/// # #[cfg(feature = "project_attr")]
/// use pin_project::{project, pin_project};
/// # #[cfg(feature = "project_attr")]
/// use std::pin::Pin;
///
/// # #[cfg(feature = "project_attr")]
/// #[pin_project]
/// enum Foo<A, B, C> {
///     Tuple(#[pin] A, B),
///     Struct { field: C },
///     Unit,
/// }
///
/// # #[cfg(feature = "project_attr")]
/// impl<A, B, C> Foo<A, B, C> {
///     #[project] // Nightly does not need a dummy attribute to the function.
///     fn baz(self: Pin<&mut Self>) {
///         #[project]
///         match self.project() {
///             Foo::Tuple(x, y) => {
///                 let _: Pin<&mut A> = x;
///                 let _: &mut B = y;
///             }
///             Foo::Struct { field } => {
///                 let _: &mut C = field;
///             }
///             Foo::Unit => {}
///         }
///     }
/// }
/// ```
///
/// Enums without variants (zero-variant enums) are not supported.
///
/// See also [`project`] attribute.
///
/// [`Pin::as_mut`]: core::pin::Pin::as_mut
/// [`drop`]: Drop::drop
/// [`UnsafeUnpin`]: https://docs.rs/pin-project/0.4.0-alpha.1/pin_project/trait.UnsafeUnpin.html
/// [`project`]: ./attr.project.html
/// [`pinned_drop`]: ./attr.pinned_drop.html
#[proc_macro_attribute]
pub fn pin_project(args: TokenStream, input: TokenStream) -> TokenStream {
    pin_project::attribute(args.into(), input.into()).into()
}

// TODO: Move this doc into pin-project crate when https://github.com/rust-lang/rust/pull/62855 merged.
/// An attribute for annotating a function that implements [`Drop`].
///
/// This attribute is only needed when you wish to provide a [`Drop`]
/// impl for your type. The function annotated with `#[pinned_drop]` acts just
/// like a normal [`drop`](Drop::drop) impl, except for the fact that it takes
/// `Pin<&mut Self>`. In particular, it will never be called more than once,
/// just like [`Drop::drop`].
///
/// Example:
///
/// ```rust
/// use pin_project::{pin_project, pinned_drop};
/// use std::pin::Pin;
///
/// #[pin_project(PinnedDrop)]
/// struct Foo {
///     #[pin] field: u8
/// }
///
/// #[pinned_drop]
/// fn my_drop(foo: Pin<&mut Foo>) {
///     println!("Dropping: {}", foo.field);
/// }
///
/// fn main() {
///     Foo { field: 50 };
/// }
/// ```
///
/// See ["pinned-drop" section of `pin_project` attribute][pinned-drop] for more.
///
/// [pinned-drop]: ./attr.pin_project.html#pinned_drop
#[proc_macro_attribute]
pub fn pinned_drop(args: TokenStream, input: TokenStream) -> TokenStream {
    let _: Nothing = syn::parse_macro_input!(args);
    pinned_drop::attribute(input.into()).into()
}

// TODO: Move this doc into pin-project crate when https://github.com/rust-lang/rust/pull/62855 merged.
/// An attribute to support pattern matching.
///
/// *This attribute is available if pin-project is built with the
/// `"project_attr"` feature.*
///
/// The attribute at the expression position is not stable, so you need to use
/// a dummy `#[project]` attribute for the function.
///
/// ## Examples
///
/// The following two syntaxes are supported.
///
/// ### `let` bindings
///
/// ```rust
/// use pin_project::{pin_project, project};
/// use std::pin::Pin;
///
/// #[pin_project]
/// struct Foo<T, U> {
///     #[pin]
///     future: T,
///     field: U,
/// }
///
/// impl<T, U> Foo<T, U> {
///     #[project] // Nightly does not need a dummy attribute to the function.
///     fn baz(self: Pin<&mut Self>) {
///         #[project]
///         let Foo { future, field } = self.project();
///
///         let _: Pin<&mut T> = future;
///         let _: &mut U = field;
///     }
/// }
/// ```
///
/// ### `match` expressions
///
/// ```rust
/// use pin_project::{project, pin_project};
/// use std::pin::Pin;
///
/// #[pin_project]
/// enum Foo<A, B, C> {
///     Tuple(#[pin] A, B),
///     Struct { field: C },
///     Unit,
/// }
///
/// impl<A, B, C> Foo<A, B, C> {
///     #[project] // Nightly does not need a dummy attribute to the function.
///     fn baz(self: Pin<&mut Self>) {
///         #[project]
///         match self.project() {
///             Foo::Tuple(x, y) => {
///                 let _: Pin<&mut A> = x;
///                 let _: &mut B = y;
///             }
///             Foo::Struct { field } => {
///                 let _: &mut C = field;
///             }
///             Foo::Unit => {}
///         }
///     }
/// }
/// ```
#[cfg(feature = "project_attr")]
#[proc_macro_attribute]
pub fn project(args: TokenStream, input: TokenStream) -> TokenStream {
    let _: Nothing = syn::parse_macro_input!(args);
    project::attribute(input.into()).into()
}

#[cfg(feature = "renamed")]
lazy_static::lazy_static! {
    pub(crate) static ref PIN_PROJECT_CRATE: String = {
        proc_macro_crate::crate_name("pin-project")
            .expect("pin-project-internal was used without pin-project!")
    };
}