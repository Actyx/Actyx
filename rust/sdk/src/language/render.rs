use std::fmt::{Result, Write};

use super::*;
use chrono::{DateTime, Local, SecondsFormat, Utc};

fn render_simple_pair(w: &mut impl Write, l: &SimpleExpr, op: &'static str, r: &SimpleExpr) -> Result {
    w.write_char('(')?;
    render_simple_expr(w, l)?;
    w.write_char(' ')?;
    w.write_str(op)?;
    w.write_char(' ')?;
    render_simple_expr(w, r)?;
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
        if x.spread {
            w.write_str("...")?;
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

pub(crate) fn render_interpolation(w: &mut impl Write, e: &[SimpleExpr]) -> Result {
    w.write_char('`')?;
    for e in e {
        w.write_char('{')?;
        render_simple_expr(w, e)?;
        w.write_char('}')?;
    }
    w.write_char('`')
}

pub fn render_simple_expr(w: &mut impl Write, e: &SimpleExpr) -> Result {
    match e {
        SimpleExpr::Variable(v) => w.write_str(v),
        SimpleExpr::Indexing(i) => render_indexing(w, i),
        SimpleExpr::Number(n) => render_number(w, n),
        SimpleExpr::String(s) => render_string(w, s),
        SimpleExpr::Interpolation(s) => render_interpolation(w, s),
        SimpleExpr::Object(o) => render_object(w, o),
        SimpleExpr::Array(a) => render_array(w, a),
        SimpleExpr::Null => w.write_str("NULL"),
        SimpleExpr::Bool(b) => render_bool(b, w),
        SimpleExpr::Not(e) => {
            w.write_char('!')?;
            render_simple_expr(w, e)
        }
        SimpleExpr::Cases(v) => render_cases(v, w),
        SimpleExpr::BinOp(e) => render_simple_pair(w, &e.1, e.0.as_str(), &e.2),
        SimpleExpr::AggrOp(e) => render_unary_function(w, e.0.as_str(), &e.1),
        SimpleExpr::FuncCall(f) => render_func_call(w, f),
        SimpleExpr::SubQuery(q) => render_query(w, q),
        SimpleExpr::KeyVar(v) => write!(w, "KEY({})", v),
        SimpleExpr::KeyLiteral(k) => write!(w, "KEY({})", k),
        SimpleExpr::TimeVar(v) => write!(w, "TIME({})", v),
        SimpleExpr::TimeLiteral(t) => write!(
            w,
            "TIME({})",
            DateTime::<Utc>::from(*t)
                .with_timezone(&Local)
                .to_rfc3339_opts(SecondsFormat::Micros, true)
        ),
        SimpleExpr::Tags(t) => write!(w, "TAGS({})", t),
        SimpleExpr::App(a) => write!(w, "APP({})", a),
    }
}

fn render_func_call(w: &mut impl Write, f: &FuncCall) -> Result {
    w.write_str(&f.name)?;
    w.write_char('(')?;
    for (idx, expr) in f.args.iter().enumerate() {
        if idx > 0 {
            w.write_str(", ")?;
        }
        render_simple_expr(w, expr)?;
    }
    w.write_char(')')
}

fn render_cases(v: &NonEmptyVec<(SimpleExpr, SimpleExpr)>, w: &mut impl Write) -> Result {
    for (pred, expr) in v.iter() {
        w.write_str("CASE ")?;
        render_simple_expr(w, pred)?;
        w.write_str(" => ")?;
        render_simple_expr(w, expr)?;
        w.write_char(' ')?;
    }
    w.write_str("ENDCASE")
}

fn render_bool(b: &bool, w: &mut impl Write) -> Result {
    if *b {
        w.write_str("TRUE")
    } else {
        w.write_str("FALSE")
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
                if e.spread {
                    w.write_str("...")?;
                }
                render_simple_expr(w, e)?;
            }
            Ok(())
        }
        Operation::Aggregate(a) => {
            w.write_str("AGGREGATE ")?;
            render_simple_expr(w, a)
        }
        Operation::Limit(l) => {
            write!(w, "LIMIT {}", l)
        }
        Operation::Binding(n, e) => {
            write!(w, "LET {} := ", n)?;
            render_simple_expr(w, e)
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
        TagAtom::Tag(t) => render_string(w, t.as_ref()),
        TagAtom::Interpolation(s) => render_interpolation(w, s),
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
    match &e.source {
        Source::Events { from, order } => {
            render_tag_expr(w, from, None)?;
            if let Some(o) = *order {
                match o {
                    Order::Asc => w.write_str(" ORDER ASC")?,
                    Order::Desc => w.write_str(" ORDER DESC")?,
                    Order::StreamAsc => w.write_str(" ORDER STREAM")?,
                }
            }
        }
        Source::Array(arr) => render_array(w, arr)?,
    }

    for op in &e.ops {
        w.write_char(' ')?;
        render_operation(w, op)?;
    }

    w.write_str(" END")
}
