//! The library's own diagnostics: where they go and how much of them.

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::{Arc, Mutex};

use pipewire_flow_sys as sys;

use crate::util::{guard, lock};

/// Severity of one log message, most to least severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LogLevel {
    /// Something failed.
    Error,
    /// The library's default threshold.
    Warning,
    /// Ordinary progress.
    Info,
    /// Detail useful while debugging.
    Debug,
    /// Per-cycle detail.
    Verbose,
}

impl LogLevel {
    fn to_raw(self) -> sys::pwf_log_level {
        match self {
            LogLevel::Error => sys::PWF_LOG_ERROR,
            LogLevel::Warning => sys::PWF_LOG_WARNING,
            LogLevel::Info => sys::PWF_LOG_INFO,
            LogLevel::Debug => sys::PWF_LOG_DEBUG,
            LogLevel::Verbose => sys::PWF_LOG_VERBOSE,
        }
    }

    fn from_raw(raw: sys::pwf_log_level) -> Self {
        match raw {
            sys::PWF_LOG_ERROR => LogLevel::Error,
            sys::PWF_LOG_WARNING => LogLevel::Warning,
            sys::PWF_LOG_INFO => LogLevel::Info,
            sys::PWF_LOG_DEBUG => LogLevel::Debug,
            _ => LogLevel::Verbose,
        }
    }
}

/// One already-formatted message from the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Record<'a> {
    /// How severe the message is.
    pub level: LogLevel,
    /// Basename of the C source file that logged it.
    pub file: &'a str,
    /// Line within that file.
    pub line: i32,
    /// The formatted message, with no trailing newline.
    pub message: &'a str,
}

type LogFn = Arc<dyn Fn(Record<'_>) + Send + Sync>;

// The C callback is process-wide and carries no per-registration state, so the
// closure lives here rather than in a handle.
static SINK: Mutex<Option<LogFn>> = Mutex::new(None);

thread_local! {
    // Set while this thread runs the callback, so a message it causes is dropped.
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// Sets the minimum severity delivered; anything less severe is dropped
/// before it is formatted. The library's default is [`LogLevel::Warning`].
pub fn set_level(level: LogLevel) {
    unsafe { sys::pwf_log_set_level(level.to_raw()) };
}

/// Routes the library's diagnostics to `callback`, replacing any callback set
/// before. With none set, messages go to stderr.
///
/// This is process-wide, and the callback can run on any thread, a real-time
/// data thread included, so it must not block. Several threads may run it at
/// once, which is why it is `Fn + Sync`; keep any state in atomics or a lock.
/// A message the callback itself causes is dropped.
pub fn set_callback<F>(callback: F)
where
    F: Fn(Record<'_>) + Send + Sync + 'static,
{
    *lock(&SINK) = Some(Arc::new(callback));
    unsafe { sys::pwf_log_set_callback(Some(on_log), std::ptr::null_mut()) };
}

/// Clears the callback, sending diagnostics back to stderr.
///
/// A call already running on another thread still finishes.
pub fn clear_callback() {
    unsafe { sys::pwf_log_set_callback(None, std::ptr::null_mut()) };
    *lock(&SINK) = None;
}

unsafe extern "C" fn on_log(
    level: sys::pwf_log_level,
    file: *const c_char,
    line: c_int,
    message: *const c_char,
    _user_data: *mut c_void,
) {
    // A message logged from inside the callback would run it again without end.
    if IN_CALLBACK.with(|flag| flag.replace(true)) {
        return;
    }
    // The lock is held only to clone the closure, never while it runs.
    let callback = lock(&SINK).clone();
    if let Some(callback) = callback {
        guard(|| {
            callback(Record {
                level: LogLevel::from_raw(level),
                file: borrow(file),
                line,
                message: borrow(message),
            })
        });
    }
    IN_CALLBACK.with(|flag| flag.set(false));
}

unsafe fn borrow<'a>(s: *const c_char) -> &'a str {
    if s.is_null() {
        return "";
    }
    CStr::from_ptr(s).to_str().unwrap_or_default()
}
