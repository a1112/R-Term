use crate::{MemoryMetric, MemorySampler, SamplerError};

const METRIC: MemoryMetric = MemoryMetric::LinuxPssBytes;

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct LinuxPssSampler {
    pid: u32,
    start_time_ticks: u64,
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub struct LinuxPssSampler;

impl LinuxPssSampler {
    #[must_use]
    pub const fn metric_kind() -> MemoryMetric {
        METRIC
    }

    /// Creates a PSS sampler bound to the current identity of `pid`.
    ///
    /// # Errors
    ///
    /// Returns an explicit unsupported error off Linux. On Linux, returns a typed
    /// process, permission, I/O, or malformed-response error when identity capture
    /// fails.
    #[cfg(target_os = "linux")]
    pub fn new(pid: u32) -> Result<Self, SamplerError> {
        let start_time_ticks = read_start_time(pid)?;
        Ok(Self {
            pid,
            start_time_ticks,
        })
    }

    /// Creates a PSS sampler bound to the current identity of `pid`.
    ///
    /// # Errors
    ///
    /// Always returns `Unsupported` when compiled for a non-Linux target.
    #[cfg(not(target_os = "linux"))]
    pub fn new(_pid: u32) -> Result<Self, SamplerError> {
        Err(SamplerError::Unsupported {
            metric: METRIC,
            detail: "Linux /proc smaps_rollup is unavailable on this platform".to_owned(),
        })
    }
}

#[cfg(target_os = "linux")]
impl MemorySampler for LinuxPssSampler {
    fn metric(&self) -> MemoryMetric {
        METRIC
    }

    fn sample(&mut self) -> Result<u64, SamplerError> {
        if read_start_time(self.pid)? != self.start_time_ticks {
            return Err(SamplerError::IdentityMismatch { pid: self.pid });
        }
        let contents = read_proc_file(self.pid, "smaps_rollup")?;
        parse_linux_smaps_rollup(&contents)
    }
}

#[cfg(not(target_os = "linux"))]
impl MemorySampler for LinuxPssSampler {
    fn metric(&self) -> MemoryMetric {
        METRIC
    }

    fn sample(&mut self) -> Result<u64, SamplerError> {
        Err(SamplerError::Unsupported {
            metric: METRIC,
            detail: "Linux /proc smaps_rollup is unavailable on this platform".to_owned(),
        })
    }
}

/// Parses the `Pss:` field from Linux `/proc/<pid>/smaps_rollup` into bytes.
///
/// # Errors
///
/// Returns a malformed-response error for a missing, duplicate, invalid, or non-kB
/// field, and an overflow error when conversion from KiB to bytes exceeds `u64`.
pub fn parse_linux_smaps_rollup(contents: &str) -> Result<u64, SamplerError> {
    let mut pss_kib = None;
    for line in contents.lines() {
        let Some(rest) = line.strip_prefix("Pss:") else {
            continue;
        };
        if pss_kib.is_some() {
            return Err(malformed("duplicate Pss field"));
        }
        let mut fields = rest.split_whitespace();
        let value = fields
            .next()
            .ok_or_else(|| malformed("missing Pss value"))?;
        let unit = fields.next().ok_or_else(|| malformed("missing Pss unit"))?;
        if fields.next().is_some() || unit != "kB" {
            return Err(malformed("Pss unit must be kB"));
        }
        let parsed = value
            .parse::<u64>()
            .map_err(|_| malformed("invalid Pss value"))?;
        pss_kib = Some(parsed);
    }
    let pss_kib = pss_kib.ok_or_else(|| malformed("missing Pss field"))?;
    pss_kib
        .checked_mul(1024)
        .ok_or(SamplerError::Overflow { metric: METRIC })
}

fn malformed(detail: &str) -> SamplerError {
    SamplerError::MalformedResponse {
        metric: METRIC,
        detail: detail.to_owned(),
    }
}

#[cfg(target_os = "linux")]
fn read_start_time(pid: u32) -> Result<u64, SamplerError> {
    let stat = read_proc_file(pid, "stat")?;
    let command_end = stat
        .rfind(')')
        .ok_or_else(|| malformed("process stat command terminator is missing"))?;
    let fields = stat
        .get(command_end + 1..)
        .ok_or_else(|| malformed("process stat suffix is missing"))?;
    fields
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| malformed("process stat start time is missing"))?
        .parse::<u64>()
        .map_err(|_| malformed("process stat start time is invalid"))
}

#[cfg(target_os = "linux")]
fn read_proc_file(pid: u32, name: &str) -> Result<String, SamplerError> {
    let path = format!("/proc/{pid}/{name}");
    std::fs::read_to_string(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SamplerError::ProcessNotFound { pid },
        std::io::ErrorKind::PermissionDenied => SamplerError::AccessDenied { pid },
        _ => SamplerError::Io {
            pid,
            detail: error.to_string(),
        },
    })
}
