use fmt::Display;
use std::{ffi::OsStr, fmt, io, io::Write, iter::once, os::windows::ffi::OsStrExt};
use tracing::{
    event::Event,
    field::Field,
    field::Visit,
    span::{Attributes, Id, Record},
    Level, Metadata, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan};
use winapi::{
    shared::ntdef::{HANDLE, NULL},
    um::{
        winbase::{DeregisterEventSource, RegisterEventSourceW, ReportEventW},
        winnt::{EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE},
    },
};

const MSG_ERROR: u32 = 0xC0000001;
const MSG_WARNING: u32 = 0x80000002;
const MSG_INFO: u32 = 0x40000003;
const MSG_DEBUG: u32 = 0x40000004;
const MSG_TRACE: u32 = 0x40000005;

fn win_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

fn mk_error(error: &str) -> io::Result<Layer> {
    Err(io::Error::new(io::ErrorKind::Other, error))
}

pub struct Layer {
    handle: HANDLE,
}

/**
 * Send & Sync need to be provided due to:
 * pub enum c_void {};
 * type HANDLE = *mut c_void; https://docs.rs/winapi/0.3.9/winapi/shared/ntdef/type.HANDLE.html
 * We assume that HANDLE is thread safe, because it is not listed as unsafe here:
 * https://docs.microsoft.com/en-us/windows/win32/wsw/thread-safety
 */
unsafe impl Send for Layer {}
unsafe impl Sync for Layer {}

impl Layer {
    pub fn new(name: &str) -> io::Result<Self> {
        let wide_name = win_string(name);
        let handle = unsafe { RegisterEventSourceW(std::ptr::null_mut(), wide_name.as_ptr()) };
        if handle == NULL {
            mk_error("Failed to register event source")
        } else {
            Ok(Self { handle })
        }
    }
}

impl Drop for Layer {
    fn drop(&mut self) -> () {
        unsafe { DeregisterEventSource(self.handle) };
    }
}

impl<S> tracing_subscriber::Layer<S> for Layer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).expect("unknown span");
        let mut buf = Vec::with_capacity(256);

        let depth = span.parent().into_iter().flat_map(|x| x.scope()).count();

        write!(buf, "s{}_name: ", depth).unwrap();
        write_value(&mut buf, span.name());
        put_metadata(&mut buf, span.metadata(), Some(depth));

        attrs.record(&mut SpanVisitor::new(&mut buf, depth));

        span.extensions_mut().insert(SpanFields(buf));
    }

    fn on_record(&self, id: &Id, values: &Record, ctx: Context<S>) {
        let span = ctx.span(id).expect("unknown span");
        let depth = span.parent().into_iter().flat_map(|x| x.scope()).count();

        let mut exts = span.extensions_mut();
        let buf = &mut exts.get_mut::<SpanFields>().expect("missing fields").0;
        values.record(&mut SpanVisitor::new(buf, depth));
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        let mut buf = Vec::with_capacity(256);

        // Record span fields
        let maybe_scope = ctx
            .current_span()
            .id()
            .and_then(|id| ctx.span_scope(id).map(|x| x.from_root()));
        if let Some(scope) = maybe_scope {
            for span in scope {
                let exts = span.extensions();
                if let Some(fields) = exts.get::<SpanFields>() {
                    buf.extend_from_slice(&fields.0)
                }
            }
        }

        // Record event fields
        put_metadata(&mut buf, event.metadata(), None);
        event.record(&mut EventVisitor::new(&mut buf));

        let utf_msg = std::str::from_utf8(&buf).unwrap();
        let msg = win_string(utf_msg);
        let mut vec = vec![msg.as_ptr()];

        let (event_type, event_id) = match *event.metadata().level() {
            Level::ERROR => (EVENTLOG_ERROR_TYPE, MSG_ERROR),
            Level::WARN => (EVENTLOG_WARNING_TYPE, MSG_WARNING),
            Level::INFO => (EVENTLOG_INFORMATION_TYPE, MSG_INFO),
            Level::DEBUG => (EVENTLOG_INFORMATION_TYPE, MSG_DEBUG),
            Level::TRACE => (EVENTLOG_INFORMATION_TYPE, MSG_TRACE),
        };

        // Send event to windows event log
        unsafe {
            ReportEventW(
                self.handle,
                event_type,
                0,
                // event id == resource msg id
                event_id,
                std::ptr::null_mut(),
                vec.len() as u16,
                0,
                vec.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
    }
}
struct SpanFields(Vec<u8>);

struct SpanVisitor<'a> {
    buf: &'a mut Vec<u8>,
    depth: usize,
}

impl<'a> SpanVisitor<'a> {
    fn new(buf: &'a mut Vec<u8>, depth: usize) -> Self {
        Self { buf, depth }
    }
}

impl Visit for SpanVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(self.buf, "s{}_", self.depth).unwrap();
        write_debug(self.buf, field.name(), value);
    }
}

struct EventVisitor<'a> {
    buf: &'a mut Vec<u8>,
}

impl<'a> EventVisitor<'a> {
    fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf }
    }
}

impl Visit for EventVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write_debug(self.buf, field.name(), value);
    }
}

fn put_metadata(buf: &mut Vec<u8>, meta: &Metadata, span: Option<usize>) {
    write_name_with_value(buf, "target", meta.target(), span);
    if let Some(file) = meta.file() {
        write_name_with_value(buf, "file", file, span);
    }
    if let Some(line) = meta.line() {
        write_name_with_value(buf, "line", line, span);
    }
}

fn write_debug(buf: &mut Vec<u8>, name: &str, value: &dyn fmt::Debug) {
    writeln!(buf, "{}: {:?}", name, value).unwrap();
}

fn write_name_with_value<T>(buf: &mut Vec<u8>, name: &str, value: T, span: Option<usize>)
where
    T: Display,
{
    if let Some(n) = span {
        write!(buf, "s{}_", n).unwrap();
    }
    writeln!(buf, "{}: {}", name, value).unwrap();
}

fn write_value<T>(buf: &mut Vec<u8>, value: T)
where
    T: Display,
{
    writeln!(buf, "{}", value).unwrap();
}
