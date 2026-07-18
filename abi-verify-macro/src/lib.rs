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

use check::{check_abi, check_arity, check_arg_widths, check_full_signature, summarize_param_names, AbiCheckArgs};
use proc_macro2::Span;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, Lit, Meta, Token};

/// Environment variable pointing at the installed game's executable. Unset
/// on any machine without the game installed (CI, other developers) — the
/// check is silently skipped in that case, never breaking their build.
const EXE_PATH_ENV_VAR: &str = "WILDSKIN_LEAGUE_EXE_PATH";

/// Parsed `#[verify_abi(...)]` attribute arguments.
struct ParsedAttr {
    pattern: String,
    call_target: bool,
    expected_args: u8,
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
    let mut expected_args = None;
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
            ("expected_args", Lit::Int(lit_int)) => expected_args = Some(lit_int.base10_parse::<u8>()?),
            ("full_signature", Lit::Str(lit_str)) => full_signature = Some(lit_str.value()),
            ("pattern" | "call_target" | "expected_args" | "full_signature", _) => {
                return Err(syn::Error::new(lit.span(), format!("wrong literal type for `{ident}`")));
            }
            _ => return Err(syn::Error::new(ident.span(), format!("unknown argument `{ident}`"))),
        }
    }

    Ok(ParsedAttr {
        pattern: pattern.ok_or_else(|| syn::Error::new(span, "missing required argument `pattern`"))?,
        call_target: call_target.unwrap_or(true),
        expected_args: expected_args.ok_or_else(|| syn::Error::new(span, "missing required argument `expected_args`"))?,
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
    // Appends `param_names_summary` (when there is one) to an error message
    // from one of the two exe-dependent checks below.
    let enrich = |message: String| match &param_names_summary {
        Some(summary) => format!("{message} (declared parameters: {summary})"),
        None => message,
    };

    // Resolved once and shared by both checks below, rather than each
    // re-reading the env var independently.
    let exe_path = std::env::var(EXE_PATH_ENV_VAR).ok();
    let exe_path = exe_path.as_ref().map(std::path::Path::new);
    let check_args = AbiCheckArgs {
        pattern: &args.pattern,
        call_target: args.call_target,
        expected_args: args.expected_args,
        item_name: &item_name,
        full_signature: args.full_signature.as_deref(),
    };

    // All three checks run independently: a full-signature mismatch
    // (something inside the function's bytes changed), an arg-count
    // mismatch against the real binary (the calling convention shifted),
    // and an arity mismatch against the type's own declared parameter list
    // (the two declarations drifted out of sync with each other) are
    // meaningfully different failures, so each gets its own
    // `compile_error!` rather than being merged into one message that would
    // hide which check actually failed.
    let mut error_messages = Vec::new();
    if let Err(message) = check_abi(&check_args, exe_path) {
        error_messages.push(enrich(message));
    }
    // `check_full_signature` itself treats `full_signature: None` as a
    // trivial `Ok(())`, so this is unconditional rather than gated on
    // `args.full_signature.is_some()`.
    if let Err(message) = check_full_signature(&check_args, exe_path) {
        error_messages.push(enrich(message));
    }
    // Fourth, exe-dependent check: declared stack-argument byte widths vs.
    // what the disassembler reads them at in the real binary.
    if let Err(message) = check_arg_widths(&check_args, exe_path, &declared_widths, &param_names) {
        error_messages.push(enrich(message));
    }
    // Pure syntax, no I/O: runs unconditionally, even when
    // `EXE_PATH_ENV_VAR` is unset and the two checks above are silently
    // skipped.
    if let Some(bare_fn) = bare_fn
        && let Err(message) = check_arity(args.expected_args, bare_fn.inputs.len(), &item_name)
    {
        error_messages.push(message);
    }

    if error_messages.is_empty() {
        return item;
    }

    let compile_errors =
        error_messages.into_iter().map(|message| syn::Error::new(Span::call_site(), message).to_compile_error());
    quote::quote! { #(#compile_errors)* }.into()
}
