//! Smoke test: with `WILDSKIN_LEAGUE_EXE_PATH` unset (the normal case for
//! every machine other than a developer's with the game installed),
//! `#[verify_abi]` must be a no-op passthrough and never break the build.
//! The only assertion that matters here is that this file compiles at all.

#[abi_verify_macro::verify_abi(pattern = "E8 ? ? ? ? 48 8D 8D ? ? 00 00")]
type PushFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> i32;

// Same idea, but also specifies `full_signature` — confirms the new
// optional argument doesn't break the "env var unset -> skip both checks,
// pass through unchanged" path either.
#[abi_verify_macro::verify_abi(
    pattern = "E8 ? ? ? ? 48 8D 8D ? ? 00 00",
    full_signature = "E8 ? ? ? ? 48 8D 8D ? ? 00 00"
)]
type UpdateFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> i32;

// Confirms the macro passes through a bare-fn type whose parameters carry
// documentation-only names, without needing the game exe.
#[abi_verify_macro::verify_abi(pattern = "E8 ? ? ? ? 48 8D 8D ? ? 00 00")]
type NamedPushFn = unsafe extern "system" fn(this: *mut core::ffi::c_void, count: i32) -> i32;

#[test]
fn compiles() {
    let _ = None::<PushFn>;
    let _ = None::<UpdateFn>;
    let _ = None::<NamedPushFn>;
}
