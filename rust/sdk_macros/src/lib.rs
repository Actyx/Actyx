#[macro_use]
extern crate proc_macro_error;

use proc_macro_error::proc_macro::TokenStream;
use std::str::FromStr;
use syn::{
    parse_macro_input, Error, Expr, ExprLit, ExprRange, ExprTuple, Lit, LitStr, RangeLimits,
};

struct Args {
    lit: LitStr,
    min: usize,
    max: usize,
}

fn parse_opt_usize(boxed: &Option<Box<Expr>>, default: usize) -> Result<usize, Error> {
    let expr = match boxed {
        Some(expr) => &**expr,
        None => return Ok(default),
    };
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(i), ..
    }) = expr
    {
        i.base10_parse::<usize>()
    } else {
        Err(Error::new_spanned(boxed, ""))
    }
}

macro_rules! lit {
    ($typ:ident, $pat:ident) => {
        Expr::Lit(ExprLit { lit: Lit::$typ($pat), .. })
    };
}
macro_rules! range {
    ($from:ident, $to:ident, $limits:ident) => {
        Expr::Range(ExprRange {$from, $to, $limits, .. })
    }
}

/// This macro takes a string and a range and asserts that the string’s length
/// lies within this range. Due to the limitations of proc_macros this macro
/// must be used in type position.
///
/// ```rust
/// use actyxos_sdk_macros::assert_len;
///
/// // this is normally emitted by macro_rules
/// #[allow(dead_code)]
/// type X = assert_len!((r##"1"##, 1..=1));
/// ```
///
/// ```compile_fail
/// use actyxos_sdk_macros::assert_len;
///
/// type X = assert_len!((r##"123456"##, 1..5));
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn assert_len(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Expr);
    let parsed = if let Expr::Tuple(ExprTuple { elems, .. }) = input {
        if elems.len() == 2 {
            match (elems.first().unwrap(), elems.last().unwrap()) {
                (lit!(Str, s), range!(from, to, limits)) => {
                    let from = match parse_opt_usize(from, 0) {
                        Ok(from) => from,
                        Err(_) => abort!(from, "must range over usize values"),
                    };
                    let to = match (parse_opt_usize(to, usize::MAX), limits) {
                        (Ok(to), RangeLimits::HalfOpen(_)) => to - 1,
                        (Ok(to), RangeLimits::Closed(_)) => to,
                        _ => abort!(from, "must range over usize values"),
                    };
                    Some(Args {
                        lit: s.clone(),
                        min: from,
                        max: to,
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };
    if parsed.is_none() {
        abort_call_site!("argument must be a tuple of string and usize range")
    }
    let Args { lit, min, max } = parsed.unwrap();

    let len = lit.value().len();
    if len < min {
        emit_error!(
            lit,
            "string of length {} not allowed here, neet at least {}",
            len,
            min
        )
    }
    if len > max {
        emit_error!(
            lit,
            "string of length {} not allowed here, need at most {}",
            len,
            max
        )
    }
    TokenStream::from_str("()").unwrap()
}
