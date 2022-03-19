use crate::value::ValueKind;
use actyx_sdk::language::{AggrOp, BinOp};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum RuntimeError {
    #[display(fmt = "variable {} is not bound", _0)]
    NotBound(#[error(ignore)] String),
    #[display(fmt = "no value added")]
    NoValueYet,
    #[display(fmt = "cannot index by {}", _0)]
    NotAnIndex(#[error(ignore)] String),
    #[display(fmt = "incompatible types in {}: {} and {}", "op.as_str()", left, right)]
    TypeErrorAggrOp {
        op: AggrOp,
        left: ValueKind,
        right: ValueKind,
    },
    #[display(
        fmt = "binary operation {} cannot be applied to {} and {}",
        "op.as_str()",
        left,
        right
    )]
    TypeErrorBinOp {
        op: BinOp,
        left: ValueKind,
        right: ValueKind,
    },
}
