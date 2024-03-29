use crate::runtime::value::ValueKind;
use ax_aql::{AggrOp, BinOp};

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum RuntimeError {
    #[display(fmt = "variable `{}` is not bound", _0)]
    NotBound(#[error(ignore)] String),
    #[display(fmt = "no value added")]
    NoValueYet,
    #[display(fmt = "cannot index by {}", _0)]
    NotAnIndex(#[error(ignore)] String),
    #[display(fmt = "property `{}` not found in {}", index, in_value)]
    NotFound { index: String, in_value: ValueKind },
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
    #[display(fmt = "cannot spread a value of type {}", _0)]
    TypeErrorSpread(#[error(ignore)] ValueKind),
    #[display(fmt = "`{}` is not of type {}", value, expected)]
    TypeError { value: String, expected: ValueKind },
    #[display(fmt = "Not supported: {}", _0)]
    NotSupported(#[error(ignore)] String),
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum RuntimeFailure {
    #[display(fmt = "anti-input cannot be processed in saturated LIMIT")]
    AntiInputInLimit,
    #[display(fmt = "anti-input cannot be processed in LAST()")]
    AntiInputInLast,
    #[display(fmt = "anti-input cannot be processed in FIRST()")]
    AntiInputInFirst,
    #[display(fmt = "anti-input cannot be processed in MIN()")]
    AntiInputInMin,
    #[display(fmt = "anti-input cannot be processed in MAX()")]
    AntiInputInMax,
}
