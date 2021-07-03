use std::fmt::{Result, Write};

use super::*;

// FIXME this whole file doesn’t yet emit parens as needed

fn render_simple_pair(w: &mut impl Write, e: &(SimpleExpr, SimpleExpr), op: &'static str) -> Result {
    render_simple_expr(w, &(*e).0)?;
    w.write_char(' ')?;
    w.write_str(op)?;
    w.write_char(' ')?;
    render_simple_expr(w, &(*e).1)
}

fn render_indexing(w: &mut impl Write, e: &Indexing) -> Result {
    render_simple_expr(w, &e.head)?;
    for t in &e.tail {
        write!(w, "{}", t)?;
    }
    Ok(())
}

fn render_number(w: &mut impl Write, e: &Number) -> Result {
    match e {
        Number::Decimal(d) => render_to_string(w, d),
        Number::Natural(n) => render_to_string(w, n),
    }
}

fn render_to_string<W: Write, T: ToString>(w: &mut W, e: &T) -> Result {
    w.write_str(&e.to_string())
}

fn render_object(w: &mut impl Write, e: &Object) -> Result {
    w.write_str("{ ")?;
    for (i, (k, v)) in e.props.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        w.write_str(k)?;
        w.write_str(": ")?;
        render_simple_expr(w, v)?;
    }
    w.write_str(" }")
}

fn render_array(w: &mut impl Write, e: &Array) -> Result {
    w.write_char('[')?;
    for (i, x) in e.items.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        render_simple_expr(w, x)?;
    }
    w.write_char(']')
}

fn render_string(w: &mut impl Write, e: &str) -> Result {
    w.write_char('\'')?;
    w.write_str(&e.replace('\'', "''"))?;
    w.write_char('\'')
}

pub fn render_simple_expr(w: &mut impl Write, e: &SimpleExpr) -> Result {
    match e {
        SimpleExpr::Var(v) => w.write_str(v),
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
            for e in s {
                if first {
                    first = false;
                } else {
                    w.write_str(", ")?;
                }
                render_simple_expr(w, e)?;
            }
            Ok(())
        }
    }
}

pub fn render_tag_expr(w: &mut impl Write, e: &TagExpr, parent: Option<&TagExpr>) -> Result {
    match e {
        TagExpr::Or(or) => {
            let wrap = matches!(parent, Some(&TagExpr::And(_)));
            if wrap {
                w.write_char('(')?;
            }
            render_tag_expr(w, &or.0, Some(e))?;
            w.write_str(" | ")?;
            render_tag_expr(w, &or.1, Some(e))?;
            if wrap {
                w.write_char(')')?;
            }
            Ok(())
        }
        TagExpr::And(and) => {
            render_tag_expr(w, &and.0, Some(e))?;
            w.write_str(" & ")?;
            render_tag_expr(w, &and.1, Some(e))
        }
        TagExpr::Atom(atom) => render_tag_atom(w, atom),
    }
}

fn render_timestamp(w: &mut impl Write, e: Timestamp) -> Result {
    use chrono::prelude::*;
    let dt: DateTime<Utc> = e.into();
    let str = if dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 && dt.nanosecond() == 0 {
        dt.format("%Y-%m-%d").to_string()
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
        TagAtom::FromLamport(fl) => {
            w.write_str("from(")?;
            let l: u64 = (*fl).into();
            render_to_string(w, &l)?;
            w.write_char(')')
        }
        TagAtom::ToLamport(tl) => {
            w.write_str("to(")?;
            let l: u64 = (*tl).into();
            render_to_string(w, &l)?;
            w.write_char(')')
        }
        TagAtom::AppId(app_id) => w.write_fmt(format_args!("appId({})", app_id)),
    }
}

pub fn render_query(w: &mut impl Write, e: &Query) -> Result {
    w.write_str("FROM ")?;
    render_tag_expr(w, &e.from, None)?;
    for op in &e.ops {
        w.write_char(' ')?;
        render_operation(w, op)?;
    }

    w.write_str(" END")
}
