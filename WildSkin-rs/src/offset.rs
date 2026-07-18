//! Vendored from the `offsetter` crate (MIT, v1.1.1,
//! <https://crates.io/crates/offsetter>) rather than kept as a dependency —
//! it's a single small `macro_rules!`, not worth a crate plus its own `paste`
//! dependency.
//!
//! Defines a `#[repr(C)]` struct with automatic `_pad<field>: [u8; N]`
//! fields inserted between named fields so each one lands at an explicit
//! byte offset — see `sdk::character_data`/`sdk::champion`/`sdk::game_state`
//! for real usage:
//!
//! ```ignore
//! offset!(
//!     pub struct Foo {
//!         0x0 => a: u32,
//!         0x8 => b: u32,
//!     }
//! );
//! ```
//!
//! The `OFFSET =>` form (rather than the original crate's bare
//! `OFFSET field: Type`) is for tooling: rustfmt/rust-analyzer can't format
//! or highlight a number directly followed by an identifier, but they handle
//! the match-arm-shaped `OFFSET => field: Type` fine.
//!
//! Single-bound (`<T>`, `<T, U>`, ...) generic structs are supported (see
//! `sdk::primitives::ManagerTemplate`), with two differences:
//!
//! - `size_of::<Ty>()` can't appear in the generated padding computation when
//!   `Ty` depends on a generic parameter ("generic parameters may not be used
//!   in const operations"; `generic_const_exprs` would lift this but is
//!   unstable). Raw-pointer fields are auto-detected and sidestep it — every
//!   pointer is `size_of::<usize>()` regardless of pointee. Any other
//!   generic-dependent field (none exist here today) needs an explicit
//!   `= SIZE_EXPR` annotation (`0x8 => list: SomeGenericThing<T> = 16,`).
//! - The per-field alignment assertion is skipped for every field — same
//!   reason, and one per-struct rule beats tracking which fields need it.

#[macro_export]
macro_rules! offset {
    (@guard ($current_offset:expr,) -> {$(#[$attr:meta])* $vis:vis struct $name:ident $(($offset:expr, $amount:expr, $fvis:vis $id:ident: $ty:ty))*}) => {
        ::paste::paste! {
            #[repr(C)]
            $(#[$attr])*
            $vis struct $name {
                $(
                    [<_pad $id>]: [u8; $amount],
                    $fvis $id: $ty
                ),*
            }
        }

        $(
            const _: () = assert!(
                $offset % core::mem::align_of::<$ty>() == 0,
                concat!(
                    "field `", stringify!($id), "` at offset ",
                    stringify!($offset),
                    " violates alignment of type `", stringify!($ty), "`"
                )
            );
        )*
    };

    (@guard ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guard ($offset + core::mem::size_of::<$ty>(), $($next)*) -> {$($output)* ($offset, $offset - ($current_offset), $fvis $id: $ty)});
    };

    (@guard ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty) -> {$($output:tt)*}) => {
        $crate::offset!(@guard ($offset + core::mem::size_of::<$ty>(),) -> {$($output)* ($offset, $offset - ($current_offset), $fvis $id: $ty)});
    };

    ($(#[$attr:meta])* $vis:vis struct $name:ident { $($input:tt)* }) => {
        $crate::offset!(@guard (0, $($input)*) -> {$(#[$attr])* $vis struct $name});
    };

    // Generic-struct variants: identical offset/padding mechanics on a
    // separate `@guardgen` tag, so the non-generic path above stays
    // untouched. Only difference: no per-field alignment assertion.
    (@guardgen ($current_offset:expr,) -> {$(#[$attr:meta])* $vis:vis struct $name:ident<$($gen:ident),+> $(($amount:expr, $fvis:vis $id:ident: $ty:ty))*}) => {
        ::paste::paste! {
            #[repr(C)]
            $(#[$attr])*
            $vis struct $name<$($gen),+> {
                $(
                    [<_pad $id>]: [u8; $amount],
                    $fvis $id: $ty
                ),*
            }
        }
    };

    // Auto-detected raw-pointer fields, matched by literal `*`/`mut`/`const`
    // tokens ahead of the `= $size:expr` arms below, so a pointer-typed field
    // never needs a size annotation — every pointer is `size_of::<usize>()`.
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: * mut $pointee:ty, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<usize>(), $($next)*) -> {$($output)* ($offset - ($current_offset), $fvis $id: *mut $pointee)});
    };
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: * mut $pointee:ty) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<usize>(),) -> {$($output)* ($offset - ($current_offset), $fvis $id: *mut $pointee)});
    };
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: * const $pointee:ty, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<usize>(), $($next)*) -> {$($output)* ($offset - ($current_offset), $fvis $id: *const $pointee)});
    };
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: * const $pointee:ty) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<usize>(),) -> {$($output)* ($offset - ($current_offset), $fvis $id: *const $pointee)});
    };

    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty = $size:expr, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + $size, $($next)*) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty = $size:expr) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + $size,) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    // Fallback for a field whose type doesn't mention the generic parameter
    // (tried last, so the arms above always win when they apply): infers the
    // size via `size_of::<$ty>()`, fine precisely because `$ty` isn't
    // generic-dependent.
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<$ty>(), $($next)*) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<$ty>(),) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    ($(#[$attr:meta])* $vis:vis struct $name:ident<$($gen:ident),+> { $($input:tt)* }) => {
        $crate::offset!(@guardgen (0, $($input)*) -> {$(#[$attr])* $vis struct $name<$($gen),+>});
    };
}
