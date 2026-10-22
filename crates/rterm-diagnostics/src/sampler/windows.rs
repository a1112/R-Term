use std::fmt::{self, Debug, Formatter};

use crate::{MemoryMetric, MemorySampler, SamplerError};

const METRIC: MemoryMetric = MemoryMetric::WindowsPrivateWorkingSetBytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsProcessSnapshot {
    pub creation_time_100ns: u64,
    pub private_working_set_bytes: Option<u64>,
    pub private_usage_bytes: u64,
    pub working_set_bytes: u64,
}

pub trait WindowsProcessQuery {
    /// Queries one identity-bound process memory snapshot.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the process is missing/inaccessible, the EX2
    /// counters are unsupported, or Windows returns an invalid response.
    fn snapshot(&mut self, pid: u32) -> Result<WindowsProcessSnapshot, SamplerError>;
}

pub struct WindowsPrivateWorkingSetSampler {
    pid: u32,
    creation_time_100ns: u64,
    query: Box<dyn WindowsProcessQuery>,
}

impl Debug for WindowsPrivateWorkingSetSampler {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsPrivateWorkingSetSampler")
            .field("pid", &self.pid)
            .field("creation_time_100ns", &self.creation_time_100ns)
            .finish_non_exhaustive()
    }
}

impl WindowsPrivateWorkingSetSampler {
    #[must_use]
    pub const fn metric_kind() -> MemoryMetric {
        METRIC
    }

    /// Creates a sampler using the native Windows process query.
    ///
    /// # Errors
    ///
    /// Returns a typed unsupported error off Windows, or a native process/query error
    /// when initial identity and EX2 capability capture fails.
    pub fn new(pid: u32) -> Result<Self, SamplerError> {
        Self::with_query(pid, NativeWindowsProcessQuery)
    }

    /// Creates a sampler with an injectable Windows query implementation.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the initial identity/counter query fails or does not
    /// include the EX2 private-working-set field.
    pub fn with_query(
        pid: u32,
        mut query: impl WindowsProcessQuery + 'static,
    ) -> Result<Self, SamplerError> {
        let initial = query.snapshot(pid)?;
        require_private_working_set(initial.private_working_set_bytes)?;
        Ok(Self {
            pid,
            creation_time_100ns: initial.creation_time_100ns,
            query: Box::new(query),
        })
    }
}

impl MemorySampler for WindowsPrivateWorkingSetSampler {
    fn metric(&self) -> MemoryMetric {
        METRIC
    }

    fn sample(&mut self) -> Result<u64, SamplerError> {
        let snapshot = self.query.snapshot(self.pid)?;
        if snapshot.creation_time_100ns != self.creation_time_100ns {
            return Err(SamplerError::IdentityMismatch { pid: self.pid });
        }
        require_private_working_set(snapshot.private_working_set_bytes)
    }
}

fn require_private_working_set(value: Option<u64>) -> Result<u64, SamplerError> {
    value.ok_or_else(|| SamplerError::Unsupported {
        metric: METRIC,
        detail: "PROCESS_MEMORY_COUNTERS_EX2.PrivateWorkingSetSize is unavailable".to_owned(),
    })
}

#[derive(Debug, Clone, Copy)]
struct NativeWindowsProcessQuery;

#[cfg(not(target_os = "windows"))]
impl WindowsProcessQuery for NativeWindowsProcessQuery {
    fn snapshot(&mut self, _pid: u32) -> Result<WindowsProcessSnapshot, SamplerError> {
        Err(SamplerError::Unsupported {
            metric: METRIC,
            detail: "Windows process memory APIs are unavailable on this platform".to_owned(),
        })
    }
}

#[cfg(target_os = "windows")]
impl WindowsProcessQuery for NativeWindowsProcessQuery {
    fn snapshot(&mut self, pid: u32) -> Result<WindowsProcessSnapshot, SamplerError> {
        native::query_process(pid)
    }
}

#[cfg(target_os = "windows")]
mod native {
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, GetLastError, HANDLE};
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    use super::{METRIC, WindowsProcessSnapshot};
    use crate::SamplerError;

    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_INVALID_HANDLE: u32 = 6;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const ERROR_NOT_FOUND: u32 = 1168;

    pub(super) fn query_process(pid: u32) -> Result<WindowsProcessSnapshot, SamplerError> {
        // SAFETY: `OpenProcess` is called with a concrete PID, no inherited handle, and
        // query-only access. A non-null handle is immediately wrapped for deterministic
        // `CloseHandle` ownership.
        let raw_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if raw_handle.is_null() {
            return Err(map_open_error(pid, last_error()));
        }
        let handle = OwnedHandle(raw_handle);
        let creation_time_100ns = process_creation_time(pid, handle.0)?;

        let mut counters = PROCESS_MEMORY_COUNTERS_EX2::default();
        let counter_size = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX2>())
            .map_err(|_| SamplerError::Overflow { metric: METRIC })?;
        counters.cb = counter_size;
        // SAFETY: `counters` is a valid, writable EX2 allocation; its first fields are
        // layout-compatible with `PROCESS_MEMORY_COUNTERS`, and `counter_size` is the
        // exact EX2 allocation size required by `K32GetProcessMemoryInfo`.
        let succeeded = unsafe {
            K32GetProcessMemoryInfo(
                handle.0,
                (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
                counter_size,
            )
        };
        if succeeded == 0 {
            return Err(map_memory_error(pid, last_error()));
        }

        Ok(WindowsProcessSnapshot {
            creation_time_100ns,
            private_working_set_bytes: Some(to_u64(counters.PrivateWorkingSetSize)?),
            private_usage_bytes: to_u64(counters.PrivateUsage)?,
            working_set_bytes: to_u64(counters.WorkingSetSize)?,
        })
    }

    fn process_creation_time(pid: u32, handle: HANDLE) -> Result<u64, SamplerError> {
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        // SAFETY: all four output pointers refer to initialized, writable `FILETIME`
        // values and `handle` remains owned/alive for the duration of the call.
        let succeeded = unsafe {
            GetProcessTimes(
                handle,
                &raw mut creation,
                &raw mut exit,
                &raw mut kernel,
                &raw mut user,
            )
        };
        if succeeded == 0 {
            return Err(map_query_error(pid, last_error()));
        }
        Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
    }

    fn to_u64(value: usize) -> Result<u64, SamplerError> {
        u64::try_from(value).map_err(|_| SamplerError::Overflow { metric: METRIC })
    }

    fn map_open_error(pid: u32, code: u32) -> SamplerError {
        match code {
            ERROR_ACCESS_DENIED => SamplerError::AccessDenied { pid },
            ERROR_INVALID_PARAMETER | ERROR_NOT_FOUND => SamplerError::ProcessNotFound { pid },
            _ => native_error(pid, "OpenProcess", code),
        }
    }

    fn map_query_error(pid: u32, code: u32) -> SamplerError {
        match code {
            ERROR_ACCESS_DENIED => SamplerError::AccessDenied { pid },
            ERROR_INVALID_HANDLE | ERROR_NOT_FOUND => SamplerError::ProcessNotFound { pid },
            _ => native_error(pid, "GetProcessTimes", code),
        }
    }

    fn map_memory_error(pid: u32, code: u32) -> SamplerError {
        match code {
            ERROR_ACCESS_DENIED => SamplerError::AccessDenied { pid },
            ERROR_INVALID_HANDLE | ERROR_NOT_FOUND => SamplerError::ProcessNotFound { pid },
            ERROR_INVALID_PARAMETER | ERROR_INSUFFICIENT_BUFFER => SamplerError::Unsupported {
                metric: METRIC,
                detail: format!("PROCESS_MEMORY_COUNTERS_EX2 query failed with OS error {code}"),
            },
            _ => native_error(pid, "K32GetProcessMemoryInfo", code),
        }
    }

    fn native_error(pid: u32, operation: &str, code: u32) -> SamplerError {
        SamplerError::Io {
            pid,
            detail: format!("{operation} failed with OS error {code}"),
        }
    }

    fn last_error() -> u32 {
        // SAFETY: `GetLastError` takes no pointers and reads the calling thread's error
        // slot immediately after a failed Windows API call.
        unsafe { GetLastError() }
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle is non-null, uniquely owned by this wrapper, and is
            // closed exactly once from `Drop`.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}
