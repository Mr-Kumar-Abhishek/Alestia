//! Structured multi-level logger and bounded log history buffer.
//!
//! Matches `celestia::util::Logger` (`src/celutil/logger.h` and `test/unit/logger_test.cpp`)
//! plus a bounded [`LogRingBuffer`] for `celestia.cfg` `LogSize`.

use std::collections::VecDeque;
use std::fmt;
use std::io::Write;

/// Severity levels ordered from highest priority (`Error`) to lowest (`Debug`).
///
/// Routing matches Celestia's `Logger`:
/// - `Error`, `Warning`, `Debug` -> error sink (`err`)
/// - `Info`, `Verbose` -> info sink (`log`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Fatal or non-recoverable errors (routed to `err`).
    Error = 0,
    /// Recoverable warnings (routed to `err`).
    Warning = 1,
    /// Standard informational messages (routed to `log`).
    Info = 2,
    /// Verbose diagnostic messages (routed to `log`).
    Verbose = 3,
    /// Developer debug traces (routed to `err`).
    Debug = 4,
}

/// Multi-stream logger routing messages according to [`Level`].
#[derive(Debug)]
pub struct Logger<WLog, WErr> {
    level: Level,
    log_sink: WLog,
    err_sink: WErr,
}

impl<WLog: Write, WErr: Write> Logger<WLog, WErr> {
    /// Create a [`Logger`] filtering messages above `level`.
    #[must_use]
    pub const fn new(level: Level, log_sink: WLog, err_sink: WErr) -> Self {
        Self {
            level,
            log_sink,
            err_sink,
        }
    }

    /// Configured verbosity threshold.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Update the active verbosity threshold.
    pub const fn set_level(&mut self, level: Level) {
        self.level = level;
    }

    /// Borrow the informational log sink.
    #[must_use]
    pub const fn log_sink(&self) -> &WLog {
        &self.log_sink
    }

    /// Borrow the error/warning/debug sink.
    #[must_use]
    pub const fn err_sink(&self) -> &WErr {
        &self.err_sink
    }

    /// Emit formatted arguments at `msg_level` if `msg_level <= self.level`.
    pub fn log_fmt(&mut self, msg_level: Level, args: fmt::Arguments<'_>) {
        if msg_level > self.level {
            return;
        }
        match msg_level {
            Level::Error | Level::Warning | Level::Debug => {
                let _ = self.err_sink.write_fmt(args);
            }
            Level::Info | Level::Verbose => {
                let _ = self.log_sink.write_fmt(args);
            }
        }
    }

    /// Emit an [`Level::Error`] message.
    pub fn error(&mut self, args: fmt::Arguments<'_>) {
        self.log_fmt(Level::Error, args);
    }

    /// Emit a [`Level::Warning`] message.
    pub fn warn(&mut self, args: fmt::Arguments<'_>) {
        self.log_fmt(Level::Warning, args);
    }

    /// Emit an [`Level::Info`] message.
    pub fn info(&mut self, args: fmt::Arguments<'_>) {
        self.log_fmt(Level::Info, args);
    }

    /// Emit a [`Level::Verbose`] message.
    pub fn verbose(&mut self, args: fmt::Arguments<'_>) {
        self.log_fmt(Level::Verbose, args);
    }

    /// Emit a [`Level::Debug`] message.
    pub fn debug(&mut self, args: fmt::Arguments<'_>) {
        self.log_fmt(Level::Debug, args);
    }
}

/// Bounded in-memory log ring buffer honoring `LogSize` from `celestia.cfg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRingBuffer {
    capacity: usize,
    entries: VecDeque<(Level, String)>,
}

impl LogRingBuffer {
    /// Create a ring buffer keeping at most `capacity` recent entries.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    /// Record a log entry, evicting the oldest entry if full.
    pub fn push(&mut self, level: Level, message: impl Into<String>) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((level, message.into()));
    }

    /// Number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the buffer contains zero entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over retained `(Level, &str)` entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = (Level, &str)> {
        self.entries.iter().map(|(lvl, msg)| (*lvl, msg.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take_utf8(buf: &mut Vec<u8>) -> String {
        let s = String::from_utf8(buf.clone()).expect("valid utf8");
        buf.clear();
        s
    }

    #[test]
    fn logger_info_level() {
        let mut logger = Logger::new(Level::Info, Vec::<u8>::new(), Vec::<u8>::new());

        logger.error(format_args!("number={}\n", 123));
        assert_eq!(take_utf8(&mut logger.err_sink), "number=123\n");
        assert!(logger.log_sink().is_empty());

        logger.warn(format_args!("string={}\n", "foobar"));
        assert_eq!(take_utf8(&mut logger.err_sink), "string=foobar\n");
        assert!(logger.log_sink().is_empty());

        logger.info(format_args!("hello world\n"));
        assert!(logger.err_sink().is_empty());
        assert_eq!(take_utf8(&mut logger.log_sink), "hello world\n");

        logger.verbose(format_args!("hi there\n"));
        assert!(logger.err_sink().is_empty());
        assert!(logger.log_sink().is_empty());

        logger.debug(format_args!("s={} e={}\n", 1, 'a'));
        assert!(logger.err_sink().is_empty());
        assert!(logger.log_sink().is_empty());
    }

    #[test]
    fn logger_verbose_level() {
        let mut logger = Logger::new(Level::Verbose, Vec::<u8>::new(), Vec::<u8>::new());

        logger.error(format_args!("number={}\n", 123));
        assert_eq!(take_utf8(&mut logger.err_sink), "number=123\n");

        logger.warn(format_args!("string={}\n", "foobar"));
        assert_eq!(take_utf8(&mut logger.err_sink), "string=foobar\n");

        logger.info(format_args!("hello world\n"));
        assert_eq!(take_utf8(&mut logger.log_sink), "hello world\n");

        logger.verbose(format_args!("hi there\n"));
        assert_eq!(take_utf8(&mut logger.log_sink), "hi there\n");

        logger.debug(format_args!("s={} e={}\n", 1, 'a'));
        assert!(logger.err_sink().is_empty());
    }

    #[test]
    fn logger_warning_level() {
        let mut logger = Logger::new(Level::Warning, Vec::<u8>::new(), Vec::<u8>::new());
        logger.error(format_args!("err\n"));
        assert_eq!(take_utf8(&mut logger.err_sink), "err\n");
        logger.warn(format_args!("warn\n"));
        assert_eq!(take_utf8(&mut logger.err_sink), "warn\n");
        logger.info(format_args!("info\n"));
        logger.verbose(format_args!("verb\n"));
        logger.debug(format_args!("dbg\n"));
        assert!(logger.log_sink().is_empty());
        assert!(logger.err_sink().is_empty());
    }

    #[test]
    fn logger_error_level() {
        let mut logger = Logger::new(Level::Error, Vec::<u8>::new(), Vec::<u8>::new());
        logger.error(format_args!("err\n"));
        assert_eq!(take_utf8(&mut logger.err_sink), "err\n");
        logger.warn(format_args!("warn\n"));
        logger.info(format_args!("info\n"));
        logger.verbose(format_args!("verb\n"));
        logger.debug(format_args!("dbg\n"));
        assert!(logger.log_sink().is_empty());
        assert!(logger.err_sink().is_empty());
    }

    #[test]
    fn logger_debug_level() {
        let mut logger = Logger::new(Level::Debug, Vec::<u8>::new(), Vec::<u8>::new());
        logger.error(format_args!("err\n"));
        assert_eq!(take_utf8(&mut logger.err_sink), "err\n");
        logger.warn(format_args!("warn\n"));
        assert_eq!(take_utf8(&mut logger.err_sink), "warn\n");
        logger.info(format_args!("info\n"));
        assert_eq!(take_utf8(&mut logger.log_sink), "info\n");
        logger.verbose(format_args!("verb\n"));
        assert_eq!(take_utf8(&mut logger.log_sink), "verb\n");
        logger.debug(format_args!("s={} e={}\n", 1, 'a'));
        assert_eq!(take_utf8(&mut logger.err_sink), "s=1 e=a\n");
        assert!(logger.log_sink().is_empty());
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut ring = LogRingBuffer::new(2);
        ring.push(Level::Info, "one");
        ring.push(Level::Warning, "two");
        ring.push(Level::Error, "three");
        assert_eq!(ring.len(), 2);
        let collected: Vec<_> = ring.iter().collect();
        assert_eq!(
            collected,
            vec![(Level::Warning, "two"), (Level::Error, "three")]
        );
    }
}
