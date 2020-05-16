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

#[macro_use]
extern crate proc_macro_error;

use proc_macro_error::proc_macro::TokenStream;
use std::str::FromStr;
use syn::{
    export::Span, parse::Parser, punctuated::Punctuated, Error, Expr, ExprLit, ExprRange, Lit,
    LitByteStr, LitStr, RangeLimits, Token,
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
    from: &Option<Box<Expr>>,
    to: &Option<Box<Expr>>,
    limits: &RangeLimits,
) -> (usize, usize) {
    let from = match parse_opt_usize(from, 0) {
        Ok(from) => from,
        Err(_) => abort!(from, "must range over usize values"),
    };
    let to = match (parse_opt_usize(to, usize::MAX), limits) {
        (Ok(to), RangeLimits::HalfOpen(_)) => to - 1,
        (Ok(to), RangeLimits::Closed(_)) => to,
        _ => abort!(from, "must range over usize values"),
    };
    (from, to)
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
#[proc_macro]
#[proc_macro_error]
pub fn assert_len(input: TokenStream) -> TokenStream {
    let elems = match Punctuated::<Expr, Token![,]>::parse_terminated.parse(input) {
        Ok(elems) => elems,
        Err(err) => abort!(err.span(), "{}", err),
    };
    let parsed = if elems.len() == 2 {
        match (elems.first().unwrap(), elems.last().unwrap()) {
            (lit!(Str, s), range!(from, to, limits)) => {
                let (from, to) = parse_range(from, to, limits);
                Some(Args {
                    lit: Str::Chars(s.clone()),
                    min: from,
                    max: to,
                })
            }
            (lit!(ByteStr, s), range!(from, to, limits)) => {
                let (from, to) = parse_range(from, to, limits);
                Some(Args {
                    lit: Str::Bytes(s.clone()),
                    min: from,
                    max: to,
                })
            }
            _ => None,
        }
    } else {
        None
    };
    if parsed.is_none() {
        abort_call_site!("argument must be a tuple of string and usize range")
    }
    let Args { lit, min, max } = parsed.unwrap();

    let len = lit.len();
    if len < min {
        emit_error!(
            lit.span(),
            "{} of length {} not allowed here, neet at least {}",
            lit.kind(),
            len,
            min
        )
    }
    if len > max {
        emit_error!(
            lit.span(),
            "{} of length {} not allowed here, need at most {}",
            lit.kind(),
            len,
            max
        )
    }
    TokenStream::from_str("()").unwrap()
}
