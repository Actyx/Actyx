#[macro_use]
extern crate proc_macro_error;

use proc_macro_error::proc_macro::{TokenStream, TokenTree::*};
use std::str::FromStr;

#[proc_macro]
#[proc_macro_error]
pub fn assert_minlen(input: TokenStream) -> TokenStream {
    let mut iter = input.into_iter();
    let minlen = iter
        .next()
        .map(|tt| {
            if let Literal(x) = tt {
                if let Ok(x) = usize::from_str_radix(&*x.to_string(), 10) {
                    x
                } else {
                    abort_call_site!("first argument must be decimal usize literal")
                }
            } else {
                abort_call_site!("first argument must be usize literal")
            }
        })
        .unwrap_or_else(|| abort_call_site!("macro needs two arguments"));
    let input = iter
        .next()
        .map(|tt| {
            if let Group(g) = tt {
                if iter.next().is_none() {
                    g.stream()
                } else {
                    abort!(g.span(), "extraneous third argument")
                }
            } else {
                abort!(tt.span(), "second argument must be a delimited tree")
            }
        })
        .unwrap_or_else(|| abort_call_site!("macro needs two arguments"));
    for token in input.clone() {
        match token {
            Group(group) => {
                let count = group.stream().into_iter().count();
                if count < minlen {
                    emit_error!(
                        group.span(),
                        "{:?} with {} elements not allowed here, need at least {}",
                        group.delimiter(),
                        count,
                        minlen
                    )
                }
            }
            Ident(_) => {}
            Punct(_) => {}
            Literal(literal) => {
                let mut s = literal.to_string();
                let bytes = if s.starts_with('b') {
                    s.remove(0);
                    true
                } else {
                    false
                };
                if s.starts_with('r') {
                    s.remove(0);
                }
                while s.starts_with('#') {
                    s.remove(0);
                    s.pop();
                }
                if s.starts_with('"') {
                    s.remove(0);
                    s.pop();
                    if s.len() < minlen {
                        if bytes {
                            emit_error!(
                                literal.span(),
                                "byte-string of length {} not allowed here, need at least {}",
                                s.len(),
                                minlen
                            );
                        } else {
                            emit_error!(
                                literal.span(),
                                "string of length {} not allowed here, need at least {}",
                                s.len(),
                                minlen
                            );
                        }
                    }
                }
            }
        }
    }
    TokenStream::from_str("()").unwrap()
}

#[proc_macro]
#[proc_macro_error]
pub fn assert_maxlen(input: TokenStream) -> TokenStream {
    let mut iter = input.into_iter();
    let maxlen = iter
        .next()
        .map(|tt| {
            if let Literal(x) = tt {
                if let Ok(x) = usize::from_str_radix(&*x.to_string(), 10) {
                    x
                } else {
                    abort_call_site!("first argument must be decimal usize literal")
                }
            } else {
                abort_call_site!("first argument must be usize literal")
            }
        })
        .unwrap_or_else(|| abort_call_site!("macro needs two arguments"));
    let input = iter
        .next()
        .map(|tt| {
            if let Group(g) = tt {
                if iter.next().is_none() {
                    g.stream()
                } else {
                    abort!(g.span(), "extraneous third argument")
                }
            } else {
                abort!(tt.span(), "second argument must be a delimited tree")
            }
        })
        .unwrap_or_else(|| abort_call_site!("macro needs two arguments"));
    for token in input.clone() {
        match token {
            Group(group) => {
                let count = group.stream().into_iter().count();
                if count > maxlen {
                    emit_error!(
                        group.span(),
                        "{:?} with {} elements not allowed here, need at least {}",
                        group.delimiter(),
                        count,
                        maxlen
                    )
                }
            }
            Ident(_) => {}
            Punct(_) => {}
            Literal(literal) => {
                let mut s = literal.to_string();
                let bytes = if s.starts_with('b') {
                    s.remove(0);
                    true
                } else {
                    false
                };
                if s.starts_with('r') {
                    s.remove(0);
                }
                while s.starts_with('#') {
                    s.remove(0);
                    s.pop();
                }
                if s.starts_with('"') {
                    s.remove(0);
                    s.pop();
                    if s.len() > maxlen {
                        if bytes {
                            emit_error!(
                                literal.span(),
                                "byte-string of length {} not allowed here, need at least {}",
                                s.len(),
                                maxlen
                            );
                        } else {
                            emit_error!(
                                literal.span(),
                                "string of length {} not allowed here, need at least {}",
                                s.len(),
                                maxlen
                            );
                        }
                    }
                }
            }
        }
    }
    TokenStream::from_str("()").unwrap()
}
