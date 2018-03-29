// Cadence - An extensible Statsd client for Rust!
//
// Copyright 2015-2017 TSH Labs
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::error;
use std::fmt;
use std::io;

use builder::MetricFormatter;

/// Trait for metrics to expose Statsd metric string slice representation.
///
/// Implementing metrics know how to turn themselves into one of the supported
/// types of metrics as defined in the [Statsd spec](https://github.com/b/statsd_spec).
pub trait Metric {
    fn as_metric_str(&self) -> &str;
}

/// Counters are simple values incremented or decremented by a client.
///
/// See the `Counted` trait for more information.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Counter {
    repr: String,
}

impl Counter {
    pub fn new(prefix: &str, key: &str, count: i64) -> Counter {
        MetricFormatter::counter(prefix, key, count).build()
    }
}

impl From<String> for Counter {
    fn from(s: String) -> Self {
        Counter { repr: s }
    }
}

impl Metric for Counter {
    fn as_metric_str(&self) -> &str {
        &self.repr
    }
}

/// Timers are a positive number of milliseconds between a start and end point.
///
/// Statistical distribution of timer values is often computed by the server.
///
/// See the `Timed` trait for more information.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Timer {
    repr: String,
}

impl Timer {
    pub fn new(prefix: &str, key: &str, time: u64) -> Timer {
        MetricFormatter::timer(prefix, key, time).build()
    }
}

impl From<String> for Timer {
    fn from(s: String) -> Self {
        Timer { repr: s }
    }
}

impl Metric for Timer {
    fn as_metric_str(&self) -> &str {
        &self.repr
    }
}

/// Gauges are an instantaneous value determined by the client.
///
/// See the `Gauged` trait for more information.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Gauge {
    repr: String,
}

impl Gauge {
    pub fn new(prefix: &str, key: &str, value: u64) -> Gauge {
        MetricFormatter::gauge(prefix, key, value).build()
    }
}

impl From<String> for Gauge {
    fn from(s: String) -> Self {
        Gauge { repr: s }
    }
}

impl Metric for Gauge {
    fn as_metric_str(&self) -> &str {
        &self.repr
    }
}

/// Meters measure the rate at which events occur as determined by the server.
///
/// See the `Metered` trait for more information.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Meter {
    repr: String,
}

impl Meter {
    pub fn new(prefix: &str, key: &str, value: u64) -> Meter {
        MetricFormatter::meter(prefix, key, value).build()
    }
}

impl From<String> for Meter {
    fn from(s: String) -> Self {
        Meter { repr: s }
    }
}

impl Metric for Meter {
    fn as_metric_str(&self) -> &str {
        &self.repr
    }
}

/// Histograms are values whose distribution is calculated by the server.
///
/// The distribution calculated for histograms is often similar to that of
/// timers. Histograms can be thought of as a more general (not limited to
/// timing things) form of timers.
///
/// See the `Histogrammed` trait for more information.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Histogram {
    repr: String,
}

impl Histogram {
    pub fn new(prefix: &str, key: &str, value: u64) -> Histogram {
        MetricFormatter::histogram(prefix, key, value).build()
    }
}

impl From<String> for Histogram {
    fn from(s: String) -> Self {
        Histogram { repr: s }
    }
}

impl Metric for Histogram {
    fn as_metric_str(&self) -> &str {
        &self.repr
    }
}

/// Potential categories an error from this library falls into.
#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub enum ErrorKind {
    InvalidInput,
    IoError,
}

/// Error generated by this library potentially wrapping another
/// type of error (exposed via the `Error` trait).
#[derive(Debug)]
pub struct MetricError {
    repr: ErrorRepr,
}

#[derive(Debug)]
enum ErrorRepr {
    WithDescription(ErrorKind, &'static str),
    IoError(io::Error),
}

impl MetricError {
    /// Return the kind of the error
    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            ErrorRepr::IoError(_) => ErrorKind::IoError,
            ErrorRepr::WithDescription(kind, _) => kind,
        }
    }
}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.repr {
            ErrorRepr::IoError(ref err) => err.fmt(f),
            ErrorRepr::WithDescription(_, desc) => desc.fmt(f),
        }
    }
}

impl error::Error for MetricError {
    fn description(&self) -> &str {
        match self.repr {
            ErrorRepr::IoError(ref err) => err.description(),
            ErrorRepr::WithDescription(_, desc) => desc,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.repr {
            ErrorRepr::IoError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for MetricError {
    fn from(err: io::Error) -> MetricError {
        MetricError {
            repr: ErrorRepr::IoError(err),
        }
    }
}

impl From<(ErrorKind, &'static str)> for MetricError {
    fn from((kind, desc): (ErrorKind, &'static str)) -> MetricError {
        MetricError {
            repr: ErrorRepr::WithDescription(kind, desc),
        }
    }
}

pub type MetricResult<T> = Result<T, MetricError>;

#[cfg(test)]
mod tests {
    use std::io;
    use std::error::Error;
    use super::{Counter, ErrorKind, Gauge, Histogram, Meter, Metric, MetricError, Timer};

    #[test]
    fn test_counter_to_metric_string() {
        let counter = Counter::new("my.app", "test.counter", 4);
        assert_eq!("my.app.test.counter:4|c", counter.as_metric_str());
    }

    #[test]
    fn test_timer_to_metric_string() {
        let timer = Timer::new("my.app", "test.timer", 34);
        assert_eq!("my.app.test.timer:34|ms", timer.as_metric_str());
    }

    #[test]
    fn test_gauge_to_metric_string() {
        let gauge = Gauge::new("my.app", "test.gauge", 2);
        assert_eq!("my.app.test.gauge:2|g", gauge.as_metric_str());
    }

    #[test]
    fn test_meter_to_metric_string() {
        let meter = Meter::new("my.app", "test.meter", 5);
        assert_eq!("my.app.test.meter:5|m", meter.as_metric_str());
    }

    #[test]
    fn test_histogram_to_metric_string() {
        let histogram = Histogram::new("my.app", "test.histogram", 45);
        assert_eq!("my.app.test.histogram:45|h", histogram.as_metric_str());
    }

    #[test]
    fn test_metric_error_kind_io_error() {
        let io_err = io::Error::new(io::ErrorKind::BrokenPipe, "Broken pipe");
        let our_err = MetricError::from(io_err);
        assert_eq!(ErrorKind::IoError, our_err.kind());
    }

    #[test]
    fn test_metric_error_kind_invalid_input() {
        let our_err = MetricError::from((ErrorKind::InvalidInput, "Nope"));
        assert_eq!(ErrorKind::InvalidInput, our_err.kind());
    }

    #[test]
    fn test_metric_error_description_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "Permission!");
        let our_err = MetricError::from(io_err);
        assert_eq!("Permission!", our_err.description());
    }

    #[test]
    fn test_metric_error_description_other() {
        let our_err = MetricError::from((ErrorKind::InvalidInput, "Something!"));
        assert_eq!("Something!", our_err.description());
    }

    #[test]
    fn test_metric_error_cause_io_error() {
        let io_err = io::Error::new(io::ErrorKind::TimedOut, "Timeout!");
        let our_err = MetricError::from(io_err);
        assert_eq!("Timeout!", our_err.cause().unwrap().description());
    }

    #[test]
    fn test_metric_error_cause_other() {
        let our_err = MetricError::from((ErrorKind::InvalidInput, "Nope!"));
        assert!(our_err.cause().is_none());
    }
}
