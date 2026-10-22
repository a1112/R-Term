use std::fmt::{self, Debug, Formatter};

use crate::{MemoryMetric, MemorySampler, SamplerError};

const METRIC: MemoryMetric = MemoryMetric::MacosPhysFootprintBytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacosProcessSnapshot {
    pub proc_start_abstime: u64,
    pub phys_footprint_bytes: Option<u64>,
    pub resident_size_bytes: u64,
}

pub trait MacosProcessQuery {
    /// Queries one identity-bound macOS process resource snapshot.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the process is missing/inaccessible, resource-info
    /// v4 is unsupported, or macOS returns an invalid response.
    fn snapshot(&mut self, pid: u32) -> Result<MacosProcessSnapshot, SamplerError>;
}

pub struct MacosPhysFootprintSampler {
    pid: u32,
    proc_start_abstime: u64,
    query: Box<dyn MacosProcessQuery>,
}

impl Debug for MacosPhysFootprintSampler {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacosPhysFootprintSampler")
            .field("pid", &self.pid)
            .field("proc_start_abstime", &self.proc_start_abstime)
            .finish_non_exhaustive()
    }
}

impl MacosPhysFootprintSampler {
    #[must_use]
    pub const fn metric_kind() -> MemoryMetric {
        METRIC
    }

    /// Creates a sampler using macOS `proc_pid_rusage` resource-info v4.
    ///
    /// # Errors
    ///
    /// Returns a typed unsupported error off macOS, or a native process/query error
    /// when initial identity and physical-footprint capture fails.
    pub fn new(pid: u32) -> Result<Self, SamplerError> {
        Self::with_query(pid, NativeMacosProcessQuery)
    }

    /// Creates a sampler with an injectable macOS query implementation.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the initial query fails or lacks physical footprint.
    pub fn with_query(
        pid: u32,
        mut query: impl MacosProcessQuery + 'static,
    ) -> Result<Self, SamplerError> {
        let initial = query.snapshot(pid)?;
        require_phys_footprint(initial.phys_footprint_bytes)?;
        Ok(Self {
            pid,
            proc_start_abstime: initial.proc_start_abstime,
            query: Box::new(query),
        })
    }
}

impl MemorySampler for MacosPhysFootprintSampler {
    fn metric(&self) -> MemoryMetric {
        METRIC
    }

    fn sample(&mut self) -> Result<u64, SamplerError> {
        let snapshot = self.query.snapshot(self.pid)?;
        if snapshot.proc_start_abstime != self.proc_start_abstime {
            return Err(SamplerError::IdentityMismatch { pid: self.pid });
        }
        require_phys_footprint(snapshot.phys_footprint_bytes)
    }
}

fn require_phys_footprint(value: Option<u64>) -> Result<u64, SamplerError> {
    value.ok_or_else(|| SamplerError::Unsupported {
        metric: METRIC,
        detail: "rusage_info_v4.ri_phys_footprint is unavailable".to_owned(),
    })
}

#[derive(Debug, Clone, Copy)]
struct NativeMacosProcessQuery;

#[cfg(not(target_os = "macos"))]
impl MacosProcessQuery for NativeMacosProcessQuery {
    fn snapshot(&mut self, _pid: u32) -> Result<MacosProcessSnapshot, SamplerError> {
        Err(SamplerError::Unsupported {
            metric: METRIC,
            detail: "macOS proc_pid_rusage is unavailable on this platform".to_owned(),
        })
    }
}

#[cfg(target_os = "macos")]
impl MacosProcessQuery for NativeMacosProcessQuery {
    fn snapshot(&mut self, pid: u32) -> Result<MacosProcessSnapshot, SamplerError> {
        native::query_process(pid)
    }
}

#[cfg(target_os = "macos")]
mod native {
    use std::mem::MaybeUninit;

    use super::{METRIC, MacosProcessSnapshot};
    use crate::SamplerError;

    pub(super) fn query_process(pid: u32) -> Result<MacosProcessSnapshot, SamplerError> {
        let native_pid = i32::try_from(pid).map_err(|_| SamplerError::MalformedResponse {
            metric: METRIC,
            detail: format!("pid {pid} exceeds macOS pid_t"),
        })?;
        let mut info = MaybeUninit::<libc::rusage_info_v4>::zeroed();
        // SAFETY: `info` is a correctly sized, aligned writable v4 allocation. The
        // pointer cast matches Apple's `rusage_info_t *` ABI and the allocation is not
        // read unless `proc_pid_rusage` reports success.
        let result = unsafe {
            libc::proc_pid_rusage(
                native_pid,
                libc::RUSAGE_INFO_V4,
                info.as_mut_ptr().cast::<libc::rusage_info_t>(),
            )
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return Err(map_error(pid, error.raw_os_error(), &error));
        }
        // SAFETY: a successful v4 query initialized the complete output structure.
        let info = unsafe { info.assume_init() };
        Ok(MacosProcessSnapshot {
            proc_start_abstime: info.ri_proc_start_abstime,
            phys_footprint_bytes: Some(info.ri_phys_footprint),
            resident_size_bytes: info.ri_resident_size,
        })
    }

    fn map_error(pid: u32, code: Option<i32>, error: &std::io::Error) -> SamplerError {
        match code {
            Some(code) if code == libc::ESRCH => SamplerError::ProcessNotFound { pid },
            Some(code) if code == libc::EACCES || code == libc::EPERM => {
                SamplerError::AccessDenied { pid }
            }
            Some(code) if code == libc::EINVAL || code == libc::ENOTSUP => {
                SamplerError::Unsupported {
                    metric: METRIC,
                    detail: format!("RUSAGE_INFO_V4 query failed: {error}"),
                }
            }
            _ => SamplerError::Io {
                pid,
                detail: format!("proc_pid_rusage failed: {error}"),
            },
        }
    }
}
