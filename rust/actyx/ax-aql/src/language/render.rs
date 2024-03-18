use std::fmt::{Result, Write};

use self::{
    parse_utils::Span,
    workflow::{Binding, EventMode, Participant, WorkflowStep},
};
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

fn render_array(w: &mut impl Write, e: &Arr<SpreadExpr>) -> Result {
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

pub(crate) fn render_interpolation(w: &mut impl Write, e: &Arr<SimpleExpr>) -> Result {
    w.write_char('`')?;
    for e in e.items.iter() {
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
            DateTime::<Utc>::try_from(*t)
                .expect("should have been parsed correctly")
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
        Operation::Machine(n, r, id) => {
            write!(w, "MACHINE {} ROLE {} ID ", n, r)?;
            render_string(w, id.as_str())
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
    let dt: DateTime<Utc> = e.try_into().map_err(|e| {
        tracing::error!("cannot render timestamp: {e}");
        std::fmt::Error
    })?;
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
        TagAtom::FromTime(ft, incl) => {
            let op = if *incl { ">=" } else { ">" };
            write!(w, "TIME {} ", op)?;
            render_timestamp(w, *ft)
        }
        TagAtom::ToTime(tt, incl) => {
            let op = if *incl { "<=" } else { "<" };
            write!(w, "TIME {} ", op)?;
            render_timestamp(w, *tt)
        }
        TagAtom::FromLamport(SortKey { lamport, stream }, incl) => {
            let op = if *incl { ">=" } else { ">" };
            write!(w, "KEY {} {}/{}", op, u64::from(*lamport), stream)
        }
        TagAtom::ToLamport(SortKey { lamport, stream }, incl) => {
            let op = if *incl { "<=" } else { "<" };
            write!(w, "KEY {} {}/{}", op, u64::from(*lamport), stream)
        }
        TagAtom::AppId(app_id) => w.write_fmt(format_args!("appId({})", app_id)),
    }
}

fn render_single_tag(w: &mut impl Write, e: &SingleTag) -> Result {
    match e {
        SingleTag::Tag(t) => render_string(w, t.as_ref()),
        SingleTag::Interpolation(s) => render_interpolation(w, s),
    }
}

fn render_label(w: &mut impl Write, l: &Label) -> Result {
    match l {
        Label::String(s) => {
            if is_ident(s) {
                w.write_str(s)
            } else {
                render_string(w, s)
            }
        }
        Label::Number(n) => write!(w, "{n}"),
    }
}

fn render_type_atom(w: &mut impl Write, a: &TypeAtom) -> Result {
    match a {
        TypeAtom::Null => w.write_str("NULL"),
        TypeAtom::Bool(Some(b)) => render_bool(b, w),
        TypeAtom::Bool(None) => w.write_str("BOOLEAN"),
        TypeAtom::Number(Some(n)) => write!(w, "{n}"),
        TypeAtom::Number(None) => w.write_str("NUMBER"),
        TypeAtom::Timestamp => w.write_str("TIMESTAMP"),
        TypeAtom::String(Some(s)) => render_string(w, s),
        TypeAtom::String(None) => w.write_str("STRING"),
        TypeAtom::Tuple(ts) => {
            w.write_char('[')?;
            for (i, t) in ts.iter().enumerate() {
                if i > 0 {
                    w.write_str(", ")?;
                }
                render_type(w, t)?;
            }
            w.write_char(']')
        }
        TypeAtom::Record(ts) => {
            w.write_char('{')?;
            for (i, (l, t)) in ts.iter().enumerate() {
                if i > 0 {
                    w.write_str(", ")?;
                }
                render_label(w, l)?;
                w.write_str(": ")?;
                render_type(w, t)?;
            }
            w.write_char('}')
        }
        TypeAtom::Universal => w.write_str("UNIVERSAL"),
    }
}

fn render_type(w: &mut impl Write, t: &Type) -> Result {
    match t {
        Type::NoValue => w.write_str("NO_VALUE"),
        Type::Atom(a) => render_type_atom(w, a),
        Type::Union(u) => {
            w.write_char('(')?;
            render_type(w, &u.0)?;
            w.write_str(" | ")?;
            render_type(w, &u.1)?;
            w.write_char(')')
        }
        Type::Intersection(i) => {
            w.write_char('(')?;
            render_type(w, &i.0)?;
            w.write_str(" & ")?;
            render_type(w, &i.1)?;
            w.write_char(')')
        }
        Type::Array(t) => {
            render_type(w, &t)?;
            w.write_str("[]")
        }
        Type::Dict(t) => {
            render_type(w, &t)?;
            w.write_str("{}")
        }
    }
}

macro_rules! render_duration_helper {
    ($w:ident, $d:ident, $f:literal, $unit:literal) => {
        let v = $d % $f;
        $d /= $f;
        write!($w, concat!("{}", $unit), v)?;
        if $d == 0 {
            return Ok(());
        }
    };
}

fn render_duration(w: &mut impl Write, mut d: u64) -> Result {
    render_duration_helper!(w, d, 1_000_000, "u");
    render_duration_helper!(w, d, 60, "s");
    render_duration_helper!(w, d, 60, "m");
    render_duration_helper!(w, d, 24, "h");
    render_duration_helper!(w, d, 7, "D");
    // months/years aren’t multiples of weeks, so stop here
    write!(w, "{}W", d)?;
    Ok(())
}

fn render_binding(w: &mut impl Write, b: &Span<Binding>) -> Result {
    w.write_char('{')?;
    w.write_str(b.name.as_ref())?;
    w.write_str(":")?;
    w.write_str(b.role.as_ref())?;
    w.write_str(" <- ")?;
    render_simple_expr(w, &b.value)?;
    w.write_char('}')?;
    Ok(())
}

fn render_event_step(
    w: &mut impl Write,
    mode: &EventMode,
    label: &Span<Ident>,
    participant: &Span<Ident>,
    binders: &Vec<Span<Binding>>,
) -> Result {
    match mode {
        EventMode::Return => w.write_str("RETURN ")?,
        EventMode::Fail => w.write_str("FAIL ")?,
        EventMode::Normal => (),
    }
    w.write_str(label.as_ref())?;
    w.write_str(" @ ")?;
    w.write_str(participant.as_ref())?;
    for b in binders.iter() {
        w.write_str(" ")?;
        render_binding(w, b)?;
    }
    Ok(())
}

fn render_workflow_step(w: &mut impl Write, e: &WorkflowStep) -> Result {
    match e {
        WorkflowStep::Event {
            state,
            mode,
            label,
            participant,
            binders,
        } => {
            if let Some(s) = state {
                w.write_str(s.as_ref())?;
                w.write_str(": ")?;
            }
            render_event_step(w, mode, label, participant, binders)?;
        }
        WorkflowStep::Retry { steps } => {
            w.write_str("RETRY ")?;
            render_scope(w, steps)?;
        }
        WorkflowStep::Timeout {
            micros,
            steps,
            mode,
            label,
            participant,
            binders,
        } => {
            w.write_str("TIMEOUT ")?;
            render_duration(w, **micros)?;
            render_scope(w, steps)?;
            render_event_step(w, mode, label, participant, binders)?;
        }
        WorkflowStep::Parallel { count, cases } => {
            w.write_str("PARALLEL ")?;
            render_number(w, &Num::Natural(**count))?;
            w.write_str(" {")?;
            for steps in cases.iter() {
                w.write_str(" CASE")?;
                for step in steps.iter() {
                    w.write_char(' ')?;
                    render_workflow_step(w, step)?;
                }
            }
            w.write_str(" }")?;
        }
        WorkflowStep::Call { workflow, args, cases } => {
            w.write_str("MATCH ")?;
            w.write_str(workflow.as_ref())?;
            w.write_char('(')?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    w.write_str(", ")?;
                }
                w.write_str(arg.as_ref())?;
            }
            w.write_str(") {")?;
            for (pred, steps) in cases.iter() {
                w.write_str(" CASE ")?;
                w.write_str(pred.as_ref().map(|x| x.as_ref()).unwrap_or("*"))?;
                w.write_str(" =>")?;
                for step in steps.iter() {
                    w.write_char(' ')?;
                    render_workflow_step(w, step)?;
                }
            }
            w.write_str(" }")?;
        }
        WorkflowStep::Compensate { body, with } => {
            w.write_str("COMPENSATE ")?;
            render_scope(w, body)?;
            w.write_str(" WITH ")?;
            render_scope(w, with)?;
        }
        WorkflowStep::Choice { cases } => {
            w.write_str("CHOICE {")?;
            for steps in cases.iter() {
                w.write_str(" CASE")?;
                for step in steps.iter() {
                    w.write_char(' ')?;
                    render_workflow_step(w, step)?;
                }
            }
            w.write_str(" }")?;
        }
    }
    Ok(())
}

fn render_scope(w: &mut impl Write, e: &[WorkflowStep]) -> Result {
    w.write_str("{ ")?;
    for (i, s) in e.iter().enumerate() {
        if i > 0 {
            w.write_char(' ')?;
        }
        render_workflow_step(w, s)?;
    }
    w.write_str(" }")
}

pub fn render_query(w: &mut impl Write, e: &Query) -> Result {
    for (label, (t, tags)) in e.events.iter() {
        w.write_str("EVENT ")?;
        w.write_str(label.as_ref())?;
        w.write_char(' ')?;
        render_type(w, t)?;
        if !tags.is_empty() {
            w.write_str(" TAGGED ")?;
            for (i, t) in tags.iter().enumerate() {
                if i > 0 {
                    w.write_str(", ")?;
                }
                render_single_tag(w, t)?;
            }
        }
        w.write_str("\n")?;
    }
    for wf in e.workflows.values() {
        w.write_str("WORKFLOW ")?;
        w.write_str(wf.name.as_ref())?;
        w.write_char('(')?;
        for (i, arg) in wf.args.iter().enumerate() {
            if i > 0 {
                w.write_str(", ")?;
            }
            match &**arg {
                Participant::Role(r) => {
                    w.write_str("ROLE ")?;
                    w.write_str(r.as_ref())?;
                }
                Participant::Unique(u) => {
                    w.write_str("UNIQUE ")?;
                    w.write_str(u.as_ref())?
                }
            }
        }
        w.write_str(") ")?;
        render_scope(w, &wf.steps)?;
        w.write_str("\n")?;
    }
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
