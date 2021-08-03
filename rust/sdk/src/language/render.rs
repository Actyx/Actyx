use std::fmt::{Result, Write};

use super::*;

fn render_simple_pair(w: &mut impl Write, e: &(SimpleExpr, SimpleExpr), op: &'static str) -> Result {
    w.write_char('(')?;
    render_simple_expr(w, &(*e).0)?;
    w.write_char(' ')?;
    w.write_str(op)?;
    w.write_char(' ')?;
    render_simple_expr(w, &(*e).1)?;
    w.write_char(')')
}

fn render_unary_function(w: &mut impl Write, f: &str, e: &SimpleExpr) -> Result {
    w.write_str(f)?;
    w.write_char('(')?;
    render_simple_expr(w, e)?;
    w.write_char(')')
}

fn render_index(w: &mut impl Write, e: &Index, with_dot: bool) -> Result {
    match e {
        Index::String(s) => {
            if is_ident(s) {
                if with_dot {
                    w.write_char('.')?;
                }
                w.write_str(s)
            } else {
                w.write_char('[')?;
                render_string(w, s)?;
                w.write_char(']')
            }
        }
        Index::Number(n) => write!(w, "[{}]", n),
        Index::Expr(e) => write!(w, "[({})]", e),
    }
}

fn render_indexing(w: &mut impl Write, e: &Ind) -> Result {
    if let SimpleExpr::Variable(v) = &*e.head {
        w.write_str(v)?;
    } else {
        w.write_char('(')?;
        render_simple_expr(w, &e.head)?;
        w.write_char(')')?;
    }
    for t in e.tail.iter() {
        render_index(w, t, true)?;
    }
    Ok(())
}

pub fn render_number(w: &mut impl Write, e: &Num) -> Result {
    match e {
        Num::Decimal(d) => write!(w, "{}", d),
        Num::Natural(n) => write!(w, "{}", n),
    }
}

fn render_object(w: &mut impl Write, e: &Obj) -> Result {
    w.write_str("{ ")?;
    for (i, (k, v)) in e.props.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        render_index(w, k, false)?;
        w.write_str(": ")?;
        render_simple_expr(w, v)?;
    }
    w.write_str(" }")
}

fn render_array(w: &mut impl Write, e: &Arr) -> Result {
    w.write_char('[')?;
    for (i, x) in e.items.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        render_simple_expr(w, x)?;
    }
    w.write_char(']')
}

pub(crate) fn render_string(w: &mut impl Write, e: &str) -> Result {
    w.write_char('\'')?;
    w.write_str(&e.replace('\'', "''"))?;
    w.write_char('\'')
}

pub fn render_simple_expr(w: &mut impl Write, e: &SimpleExpr) -> Result {
    match e {
        SimpleExpr::Variable(v) => w.write_str(v),
        SimpleExpr::Indexing(i) => render_indexing(w, i),
        SimpleExpr::Number(n) => render_number(w, n),
        SimpleExpr::String(s) => render_string(w, s),
        SimpleExpr::Object(o) => render_object(w, o),
        SimpleExpr::Array(a) => render_array(w, a),
        SimpleExpr::Null => w.write_str("NULL"),
        SimpleExpr::Bool(b) => {
            if *b {
                w.write_str("TRUE")
            } else {
                w.write_str("FALSE")
            }
        }
        SimpleExpr::Not(e) => {
            w.write_char('!')?;
            render_simple_expr(w, e)
        }
        SimpleExpr::Cases(v) => {
            for (pred, expr) in v.iter() {
                w.write_str("CASE ")?;
                render_simple_expr(w, pred)?;
                w.write_str(" => ")?;
                render_simple_expr(w, expr)?;
            }
            w.write_str(" ENDCASE")
        }
        SimpleExpr::Add(e) => render_simple_pair(w, e, "+"),
        SimpleExpr::Sub(e) => render_simple_pair(w, e, "-"),
        SimpleExpr::Mul(e) => render_simple_pair(w, e, "*"),
        SimpleExpr::Div(e) => render_simple_pair(w, e, "/"),
        SimpleExpr::Mod(e) => render_simple_pair(w, e, "%"),
        SimpleExpr::Pow(e) => render_simple_pair(w, e, "^"),
        SimpleExpr::And(e) => render_simple_pair(w, e, "&"),
        SimpleExpr::Or(e) => render_simple_pair(w, e, "|"),
        SimpleExpr::Xor(e) => render_simple_pair(w, e, "~"),
        SimpleExpr::Lt(e) => render_simple_pair(w, e, "<"),
        SimpleExpr::Le(e) => render_simple_pair(w, e, "<="),
        SimpleExpr::Gt(e) => render_simple_pair(w, e, ">"),
        SimpleExpr::Ge(e) => render_simple_pair(w, e, ">="),
        SimpleExpr::Eq(e) => render_simple_pair(w, e, "="),
        SimpleExpr::Ne(e) => render_simple_pair(w, e, "!="),
        SimpleExpr::Sum(e) => render_unary_function(w, "SUM", e),
        SimpleExpr::Min(e) => render_unary_function(w, "MIN", e),
        SimpleExpr::Max(e) => render_unary_function(w, "MAX", e),
        SimpleExpr::First(e) => render_unary_function(w, "FIRST", e),
        SimpleExpr::Last(e) => render_unary_function(w, "LAST", e),
    }
}

fn render_operation(w: &mut impl Write, e: &Operation) -> Result {
    match e {
        Operation::Filter(f) => {
            w.write_str("FILTER ")?;
            render_simple_expr(w, f)
        }
        Operation::Select(s) => {
            w.write_str("SELECT ")?;
            let mut first = true;
            for e in s.iter() {
                if first {
                    first = false;
                } else {
                    w.write_str(", ")?;
                }
                render_simple_expr(w, e)?;
            }
            Ok(())
        }
        Operation::Aggregate(a) => {
            w.write_str("AGGREGATE ")?;
            render_simple_expr(w, a)
        }
    }
}

pub fn render_tag_expr(w: &mut impl Write, e: &TagExpr, _parent: Option<&TagExpr>) -> Result {
    match e {
        TagExpr::Or(or) => {
            w.write_char('(')?;
            render_tag_expr(w, &or.0, Some(e))?;
            w.write_str(" | ")?;
            render_tag_expr(w, &or.1, Some(e))?;
            w.write_char(')')
        }
        TagExpr::And(and) => {
            w.write_char('(')?;
            render_tag_expr(w, &and.0, Some(e))?;
            w.write_str(" & ")?;
            render_tag_expr(w, &and.1, Some(e))?;
            w.write_char(')')
        }
        TagExpr::Atom(atom) => render_tag_atom(w, atom),
    }
}

fn render_timestamp(w: &mut impl Write, e: Timestamp) -> Result {
    use chrono::prelude::*;
    let dt: DateTime<Utc> = e.into();
    let str = if dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 && dt.nanosecond() == 0 {
        dt.format("%Y-%m-%dZ").to_string()
    } else {
        dt.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    };
    w.write_str(&str)
}

fn render_tag_atom(w: &mut impl Write, e: &TagAtom) -> Result {
    match e {
        TagAtom::Tag(t) => render_string(w, &t.to_string()),
        TagAtom::AllEvents => w.write_str("allEvents"),
        TagAtom::IsLocal => w.write_str("isLocal"),
        TagAtom::FromTime(ft) => {
            w.write_str("from(")?;
            render_timestamp(w, *ft)?;
            w.write_char(')')
        }
        TagAtom::ToTime(tt) => {
            w.write_str("to(")?;
            render_timestamp(w, *tt)?;
            w.write_char(')')
        }
        TagAtom::FromLamport(SortKey { lamport, stream }) => {
            w.write_str("from(")?;
            write!(w, "{}/{}", u64::from(*lamport), stream)?;
            w.write_char(')')
        }
        TagAtom::ToLamport(SortKey { lamport, stream }) => {
            w.write_str("to(")?;
            write!(w, "{}/{}", u64::from(*lamport), stream)?;
            w.write_char(')')
        }
        TagAtom::AppId(app_id) => w.write_fmt(format_args!("appId({})", app_id)),
    }
}

pub fn render_query(w: &mut impl Write, e: &Query) -> Result {
    if !e.features.is_empty() {
        w.write_str("FEATURES(")?;
        for (i, f) in e.features.iter().enumerate() {
            if i > 0 {
                w.write_char(' ')?;
            }
            w.write_str(f)?;
        }
        w.write_str(") ")?;
    }
    w.write_str("FROM ")?;
    render_tag_expr(w, &e.from, None)?;
    for op in &e.ops {
        w.write_char(' ')?;
        render_operation(w, op)?;
    }

    w.write_str(" END")
}
