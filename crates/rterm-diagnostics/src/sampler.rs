mod linux;
#[cfg_attr(target_os = "macos", allow(unsafe_code))]
mod macos;
#[cfg_attr(target_os = "windows", allow(unsafe_code))]
mod windows;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub use linux::{LinuxPssSampler, parse_linux_smaps_rollup};
pub use macos::{MacosPhysFootprintSampler, MacosProcessQuery, MacosProcessSnapshot};
pub use windows::{WindowsPrivateWorkingSetSampler, WindowsProcessQuery, WindowsProcessSnapshot};

use crate::MemoryMetric;

pub trait MemorySampler {
    fn metric(&self) -> MemoryMetric;

    /// Samples the configured child process using the declared native metric.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the metric is unsupported, the process is gone or
    /// inaccessible, its identity changed, or the native response is invalid.
    fn sample(&mut self) -> Result<u64, SamplerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamplerError {
    Unsupported {
        metric: MemoryMetric,
        detail: String,
    },
    ProcessNotFound {
        pid: u32,
    },
    AccessDenied {
        pid: u32,
    },
    IdentityMismatch {
        pid: u32,
    },
    MalformedResponse {
        metric: MemoryMetric,
        detail: String,
    },
    Overflow {
        metric: MemoryMetric,
    },
    Io {
        pid: u32,
        detail: String,
    },
}

impl Display for SamplerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported { metric, detail } => {
                write!(formatter, "{metric:?} is unsupported: {detail}")
            }
            Self::ProcessNotFound { pid } => write!(formatter, "process {pid} was not found"),
            Self::AccessDenied { pid } => write!(formatter, "access denied for process {pid}"),
            Self::IdentityMismatch { pid } => {
                write!(formatter, "process {pid} identity changed")
            }
            Self::MalformedResponse { metric, detail } => {
                write!(formatter, "malformed {metric:?} response: {detail}")
            }
            Self::Overflow { metric } => write!(formatter, "{metric:?} value overflowed bytes"),
            Self::Io { pid, detail } => {
                write!(formatter, "process {pid} sampling failed: {detail}")
            }
        }
    }
}

impl Error for SamplerError {}
