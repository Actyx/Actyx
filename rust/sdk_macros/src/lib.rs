/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! Supporting macros for the ActyxOS SDK
//!
//! The macros exported here are in this separate crate due to current restrictions on
//! proc_macros in Rust. Please see the [ActyxOS SDK](https://docs.rs/actyxos_sdk) for
//! more information.

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{
    export::Span,
    parse::{Parse, ParseStream},
    Error, Expr, ExprLit, ExprRange, Item, Lit, LitByteStr, LitStr, RangeLimits, Token,
};

enum Str {
    Chars(LitStr),
    Bytes(LitByteStr),
}

impl Str {
    pub fn len(&self) -> usize {
        match self {
            Str::Bytes(lit) => lit.value().len(),
            Str::Chars(lit) => lit.value().len(),
        }
    }
    pub fn span(&self) -> Span {
        match self {
            Str::Bytes(lit) => lit.span(),
            Str::Chars(lit) => lit.span(),
        }
    }
    pub fn kind(&self) -> &str {
        match self {
            Str::Bytes(_) => "byte-string",
            Str::Chars(_) => "string",
        }
    }
}

struct Args {
    lit: Str,
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

fn parse_range(
    from: Option<Box<Expr>>,
    to: Option<Box<Expr>>,
    limits: RangeLimits,
) -> Result<(usize, usize), Error> {
    let from = match parse_opt_usize(&from, 0) {
        Ok(from) => from,
        _ => return Err(Error::new_spanned(from, "must range over usize values")),
    };
    let to = match (parse_opt_usize(&to, usize::MAX), limits) {
        (Ok(to), RangeLimits::HalfOpen(_)) => to - 1,
        (Ok(to), RangeLimits::Closed(_)) => to,
        _ => return Err(Error::new_spanned(from, "must range over usize values")),
    };
    Ok((from, to))
}

macro_rules! lit {
    ($typ:ident, $pat:ident) => {
        Expr::Lit(ExprLit { lit: Lit::$typ($pat), .. })
    };
}
macro_rules! range {
    ($from:ident, $to:ident, $limits:ident) => {
        ExprRange {$from, $to, $limits, .. }
    }
}

/// This macro takes a string and a range and asserts that the string’s length
/// lies within this range. Due to the limitations of proc_macros this macro
/// must be used in type position (for the simple check with two arguments), or
/// it must be used in item position (for the extended mode shown below).
///
/// This works:
///
/// ```rust
/// use actyxos_sdk_macros::assert_len;
///
/// // this is normally emitted by macro_rules
/// #[allow(dead_code)]
/// type X = assert_len!(b"A", 1..=1);
/// # type Y = assert_len!(r#"A"#, 1..2);
/// ```
///
/// This does not compile:
///
/// ```compile_fail
/// use actyxos_sdk_macros::assert_len;
///
/// type X = assert_len!(r##"123456"##, ..5);
/// ```
///
/// It is possible to only perform the length check if the argument is a (byte)string literal
/// and emit transformation code depending on whether it was a literal.
/// Due to the restriction on procedural macros (they cannot expand to expressions or statements)
/// we need to wrap the resulting logic in top-level items as shown below:
///
/// ```rust
/// macro_rules! transform {
///     ($expr:tt) => {{
///         mod y {
///             actyxos_sdk_macros::assert_len! {
///                 $expr,
///                 1..5,
///                 pub fn x() -> usize { $expr.len() }, // it was a string literal
///                 pub fn x() -> String { format!("{}", $expr) } // it was something else
///             }
///         }
///         y::x()
///     }};
/// }
///
/// assert_eq!(transform!("helo"), 4);
/// assert_eq!(transform!(("hello")), "hello");
/// ```
///
/// One drawback of this approach is that we need to match a TokenTree (tt) in the
/// pattern because otherwise `assert_len!` won’t see the actual string literals,
/// which implies that any expression that consists of more than one token will need
/// to be wrapped in parentheses.
#[proc_macro]
pub fn assert_len(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match assert_len_impl(input) {
        Ok(res) => res.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct Inputs {
    literal: Expr,
    range: ExprRange,
    first: Option<Item>,
    second: Option<Item>,
}

impl Parse for Inputs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let literal: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let range: ExprRange = input.parse()?;
        let mut first: Option<Item> = None;
        let mut second: Option<Item> = None;
        if input.parse::<Token![,]>().is_err() {
            return Ok(Inputs {
                literal,
                range,
                first,
                second,
            });
        }
        first = Some(input.parse()?);
        input.parse::<Token![,]>()?;
        second = Some(input.parse()?);
        Ok(Inputs {
            literal,
            range,
            first,
            second,
        })
    }
}

fn assert_len_impl(input: proc_macro::TokenStream) -> Result<TokenStream, Error> {
    let Inputs {
        literal,
        range,
        first,
        second,
    } = syn::parse(input)?;

    let parsed = match (literal, range) {
        (lit!(Str, s), range!(from, to, limits)) => {
            let (from, to) = parse_range(from, to, limits)?;
            Some(Args {
                lit: Str::Chars(s),
                min: from,
                max: to,
            })
        }
        (lit!(ByteStr, s), range!(from, to, limits)) => {
            let (from, to) = parse_range(from, to, limits)?;
            Some(Args {
                lit: Str::Bytes(s),
                min: from,
                max: to,
            })
        }
        _ => None,
    };
    if parsed.is_none() {
        if let Some(second) = second {
            return Ok(second.into_token_stream());
        }
        return Err(Error::new(
            Span::call_site(),
            "argument must be a tuple of string and usize range",
        ));
    }
    let Args { lit, min, max } = parsed.unwrap();

    let len = lit.len();
    let mut error = TokenStream::new();
    if len < min {
        error.extend(
            Error::new(
                lit.span(),
                format!(
                    "{} of length {} not allowed here, neet at least {}",
                    lit.kind(),
                    len,
                    min
                ),
            )
            .to_compile_error(),
        );
    }
    if len > max {
        error.extend(
            Error::new(
                lit.span(),
                format!(
                    "{} of length {} not allowed here, need at most {}",
                    lit.kind(),
                    len,
                    max
                ),
            )
            .to_compile_error(),
        );
    }
    Ok(first
        .map(|f| {
            let first = f.into_token_stream();
            // must emit compile_error! macro invocation before the item to avoid warnings in case of errors
            quote!(#error #first)
        })
        .unwrap_or_else(|| if error.is_empty() { quote!(()) } else { error }))
}
