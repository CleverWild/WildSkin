//! `#[verify_abi]`: an attribute for FFI function-pointer `type` aliases that
//! checks the declared parameter count against what analysis of the real
//! game binary on disk recovers, catching ABI drift (a game patch adding or
//! removing a parameter) at compile time instead of at runtime as a crash.
//! Optionally (via `full_signature = "..."`), it also checks the function's
//! actual bytes against a developer-recorded byte template — a second,
//! independent, stricter check that catches changes the arg-count heuristic
//! alone can't see (e.g. the locator pattern coincidentally matching a
//! different function after a patch).
//!
//! All the real logic lives in the plain, unit-tested [`check`] module; this
//! file is just token-stream plumbing around it.

mod check;

use abi_verify::{verification_disabled, DISABLED_ENV_VAR, EXE_PATH_ENV_VAR};
use check::{check_abi, check_arg_widths, check_full_signature, summarize_param_names, AbiCheckArgs};
use proc_macro2::Span;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, Lit, Meta, Token};

/// Keeps the "game not found" notice to once per crate compilation.
static GAME_NOT_FOUND_WARNED: AtomicBool = AtomicBool::new(false);

// `eprintln!`, not a lint: a promotable warning would break any `-D warnings` build that just has no game installed.
fn warn_game_not_found() {
    if !GAME_NOT_FOUND_WARNED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "warning: abi-verify: no game executable found ({EXE_PATH_ENV_VAR} is unset and none was \
             auto-detected at a standard install path) — binary ABI checks skipped; set {EXE_PATH_ENV_VAR} \
             to your game exe to enable them, or set {DISABLED_ENV_VAR}=true to silence this."
        );
    }
}

/// Parsed `#[verify_abi(...)]` attribute arguments.
struct ParsedAttr {
    pattern: String,
    call_target: bool,
    /// Developer-recorded byte template, from an optional `full_signature =
    /// "..."` argument. Absent by default: not every annotated item needs
    /// this stricter check, and existing usages shouldn't have to opt in.
    full_signature: Option<String>,
}

fn parse_args(attr: proc_macro2::TokenStream) -> syn::Result<ParsedAttr> {
    let span = attr.span();
    let pairs = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr)?;

    let mut pattern = None;
    let mut call_target = None;
    let mut full_signature = None;

    for meta in pairs {
        let Meta::NameValue(name_value) = &meta else {
            return Err(syn::Error::new(meta.span(), "expected `name = value`"));
        };
        let Some(ident) = name_value.path.get_ident() else {
            return Err(syn::Error::new(name_value.path.span(), "expected a plain identifier"));
        };
        let Expr::Lit(ExprLit { lit, .. }) = &name_value.value else {
            return Err(syn::Error::new(name_value.value.span(), "expected a literal value"));
        };

        match (ident.to_string().as_str(), lit) {
            ("pattern", Lit::Str(lit_str)) => pattern = Some(lit_str.value()),
            ("call_target", Lit::Bool(lit_bool)) => call_target = Some(lit_bool.value),
            ("full_signature", Lit::Str(lit_str)) => full_signature = Some(lit_str.value()),
            ("pattern" | "call_target" | "full_signature", _) => {
                return Err(syn::Error::new(lit.span(), format!("wrong literal type for `{ident}`")));
            }
            // Removed: the count is read off the annotated type's own parameter list.
            ("expected_args", _) => {
                return Err(syn::Error::new(ident.span(), "`expected_args` is obsolete — delete it"));
            }
            _ => return Err(syn::Error::new(ident.span(), format!("unknown argument `{ident}`"))),
        }
    }

    Ok(ParsedAttr {
        pattern: pattern.ok_or_else(|| syn::Error::new(span, "missing required argument `pattern`"))?,
        call_target: call_target.unwrap_or(true),
        full_signature,
    })
}

/// Byte width of a declared parameter's Rust type, or `None` for a type we
/// can't confidently map to a concrete size (skipped by the width check).
/// Pointers, references, and fn-pointers are all 8 bytes on this project's
/// only target (x86-64 Windows).
fn type_to_width(ty: &syn::Type) -> Option<u8> {
    match ty {
        syn::Type::Ptr(_) | syn::Type::Reference(_) | syn::Type::BareFn(_) => Some(8),
        syn::Type::Path(type_path) => {
            let ident = type_path.path.segments.last()?.ident.to_string();
            match ident.as_str() {
                "usize" | "isize" | "u64" | "i64" | "f64" => Some(8),
                "u32" | "i32" | "f32" => Some(4),
                "u16" | "i16" => Some(2),
                "u8" | "i8" | "bool" => Some(1),
                _ => None,
            }
        }
        _ => None,
    }
}

#[proc_macro_attribute]
pub fn verify_abi(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Switched off entirely (CI): a pure passthrough, emitting nothing at all.
    if verification_disabled() {
        return item;
    }

    let args = match parse_args(attr.into()) {
        Ok(args) => args,
        Err(error) => return error.to_compile_error().into(),
    };

    let Ok(item_type) = syn::parse::<syn::ItemType>(item.clone()) else {
        return syn::Error::new(Span::call_site(), "#[verify_abi] can only be applied to a `type Alias = ...;` item")
            .to_compile_error()
            .into();
    };
    let item_name = item_type.ident.to_string();

    // Bare-fn parameter names are pure documentation as far as the type
    // system is concerned (`fn(this: usize)` and `fn(usize)` are the same
    // type), but `syn` still hands them to us, so they're read here for two
    // purposes: the always-on arity self-check below, and enriching the two
    // exe-dependent checks' error messages further down. Gracefully skipped
    // (rather than erroring) when the underlying type isn't a bare-fn type
    // at all, which shouldn't happen for this macro's intended use but isn't
    // this macro's job to police.
    let bare_fn = match &*item_type.ty {
        syn::Type::BareFn(bare_fn) => Some(bare_fn),
        _ => None,
    };
    let param_names: Vec<Option<String>> = bare_fn.map_or_else(Vec::new, |bare_fn| {
        bare_fn.inputs.iter().map(|input| input.name.as_ref().map(|(ident, _)| ident.to_string())).collect()
    });
    // Byte width expected for each declared parameter's Rust type, used by the
    // stack-argument width check below. `None` for any type we can't map to a
    // concrete size (that slot is then skipped, not guessed).
    let declared_widths: Vec<Option<u8>> =
        bare_fn.map_or_else(Vec::new, |bare_fn| bare_fn.inputs.iter().map(|input| type_to_width(&input.ty)).collect());
    let param_names_summary = summarize_param_names(&param_names);

    // Resolved once, shared by all three exe-dependent checks.
    let exe_path = std::env::var(EXE_PATH_ENV_VAR).ok();
    let exe_path = exe_path.as_ref().map(std::path::Path::new);
    if exe_path.is_none() {
        warn_game_not_found();
    }

    // Common footer: what was declared, which binary was read, how to steer it.
    let enrich = |message: String| {
        let mut message = message;
        if let Some(summary) = &param_names_summary {
            let _ = write!(message, "\n  declared parameters: {summary}");
        }
        if let Some(exe_path) = exe_path {
            let _ = write!(message, "\n  verified against: {}", exe_path.display());
        }
        let _ = write!(
            message,
            "\n  env: `{EXE_PATH_ENV_VAR}` chooses which binary is verified (unset = auto-detect a standard \
             install); `{DISABLED_ENV_VAR}=true` turns these checks off entirely."
        );
        message
    };
    let check_args = AbiCheckArgs {
        pattern: &args.pattern,
        call_target: args.call_target,
        declared_args: bare_fn.and_then(|bare_fn| u8::try_from(bare_fn.inputs.len()).ok()),
        item_name: &item_name,
        full_signature: args.full_signature.as_deref(),
    };

    // Each check gets its own `compile_error!` so one failure can't mask another.
    let error_messages: Vec<String> = [
        check_abi(&check_args, exe_path),
        // Unconditional: a `None` `full_signature` is a trivial `Ok(())` inside.
        check_full_signature(&check_args, exe_path),
        check_arg_widths(&check_args, exe_path, &declared_widths, &param_names),
    ]
    .into_iter()
    .filter_map(Result::err)
    .map(&enrich)
    .collect();

    if error_messages.is_empty() {
        return item;
    }

    let compile_errors =
        error_messages.into_iter().map(|message| syn::Error::new(Span::call_site(), message).to_compile_error());
    quote::quote! { #(#compile_errors)* }.into()
}
