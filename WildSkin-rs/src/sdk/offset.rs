//! Vendored from the `offsetter` crate (MIT, v1.1.1): a single small
//! `macro_rules!`, not worth a crate plus its `paste` dependency.
//!
//! Defines a `#[repr(C)]` struct with auto `_pad<field>` bytes between named
//! fields so each lands at an explicit byte offset:
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
//! The `OFFSET =>` form (vs the crate's bare `OFFSET field: Type`) is for
//! tooling: rustfmt/rust-analyzer choke on a number directly before an ident.
//!
//! Single-bound generic structs are supported (see
//! `sdk::primitives::ManagerTemplate`), with two differences:
//!
//! - `size_of::<Ty>()` can't appear in padding computation when `Ty` is
//!   generic-dependent (needs unstable `generic_const_exprs`). Raw-pointer
//!   fields are auto-detected (every pointer is `size_of::<usize>()`); any
//!   other generic-dependent field needs an explicit `= SIZE_EXPR`.
//! - Per-field alignment assertion is skipped for every field, same reason.

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

    // Generic-struct variants on a separate `@guardgen` tag. Only difference
    // from the non-generic path: no per-field alignment assertion.
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

    // Raw-pointer fields, matched before the `= $size:expr` arms so a pointer
    // field never needs a size annotation: every pointer is `size_of::<usize>()`.
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

    // Fallback (tried last) for a field whose type isn't generic-dependent:
    // infers size via `size_of::<$ty>()`.
    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty, $($next:tt)*) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<$ty>(), $($next)*) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    (@guardgen ($current_offset:expr, $offset:literal => $fvis:vis $id:ident: $ty:ty) -> {$($output:tt)*}) => {
        $crate::offset!(@guardgen ($offset + core::mem::size_of::<$ty>(),) -> {$($output)* ($offset - ($current_offset), $fvis $id: $ty)});
    };

    ($(#[$attr:meta])* $vis:vis struct $name:ident<$($gen:ident),+> { $($input:tt)* }) => {
        $crate::offset!(@guardgen (0, $($input)*) -> {$(#[$attr])* $vis struct $name<$($gen),+>});
    };

    // Vtable-method extension: `struct {..} vtable { SLOT => vis fn ..; }`
    // generates the struct plus typed virtual-call wrappers. Vtable pointer is
    // read from offset 0 (C++ polymorphic layout), so no `vtable` field needed.
    // NOTE: a slot index has no AOB to `verify_abi` against, so unlike byte
    // offsets it can silently drift between patches; document that per method.
    (
        $(#[$attr:meta])* $vis:vis struct $name:ident { $($fields:tt)* }
        vtable { $($methods:tt)* }
    ) => {
        $crate::offset!($(#[$attr])* $vis struct $name { $($fields)* });
        impl $name {
            $crate::offset!(@vtable $($methods)*);
        }
    };

    (@vtable) => {};
    (@vtable
        $(#[$mattr:meta])*
        $slot:literal => $mvis:vis fn $mname:ident(&self $(, $arg:ident: $argty:ty)*) -> $ret:ty;
        $($rest:tt)*
    ) => {
        $(#[$mattr])*
        /// # Safety
        /// Caller guarantees `self` is a live object whose vtable (at offset 0)
        /// has this slot populated with a matching `unsafe extern "system"` fn.
        #[allow(
            clippy::undocumented_unsafe_blocks,
            clippy::multiple_unsafe_ops_per_block,
            clippy::macro_metavars_in_unsafe,
            reason = "generated vtable dispatch: one logical unsafe operation (an indirect virtual call); its contract is the # Safety doc. The only metavars in the unsafe block are the slot literal and this method's own already-evaluated parameters, not caller expressions injected into unsafe"
        )]
        $mvis unsafe fn $mname(&self $(, $arg: $argty)*) -> $ret {
            let this = core::ptr::from_ref::<Self>(self);
            unsafe {
                let vtable = *this.cast::<*const usize>();
                let func_ptr = *vtable.add($slot);
                let func: unsafe extern "system" fn(usize $(, $argty)*) -> $ret =
                    core::mem::transmute(func_ptr);
                func(this as usize $(, $arg)*)
            }
        }
        $crate::offset!(@vtable $($rest)*);
    };
}
