use std::collections::VecDeque;

use rterm_diagnostics::{
    MemoryMetric, MemorySampler, SamplerError, WindowsPrivateWorkingSetSampler,
    WindowsProcessQuery, WindowsProcessSnapshot,
};

#[derive(Debug)]
struct FakeQuery {
    snapshots: VecDeque<Result<WindowsProcessSnapshot, SamplerError>>,
}

impl FakeQuery {
    fn new(
        snapshots: impl IntoIterator<Item = Result<WindowsProcessSnapshot, SamplerError>>,
    ) -> Self {
        Self {
            snapshots: snapshots.into_iter().collect(),
        }
    }
}

impl WindowsProcessQuery for FakeQuery {
    fn snapshot(&mut self, _pid: u32) -> Result<WindowsProcessSnapshot, SamplerError> {
        self.snapshots
            .pop_front()
            .expect("test query snapshot was exhausted")
    }
}

fn snapshot(identity: u64, private_working_set_bytes: Option<u64>) -> WindowsProcessSnapshot {
    WindowsProcessSnapshot {
        creation_time_100ns: identity,
        private_working_set_bytes,
        private_usage_bytes: 900,
        working_set_bytes: 800,
    }
}

#[test]
fn sampler_reports_private_working_set_without_substituting_other_counters() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(10, Some(456)))]);
    let mut sampler = WindowsPrivateWorkingSetSampler::with_query(42, query).unwrap();

    assert_eq!(
        sampler.metric(),
        MemoryMetric::WindowsPrivateWorkingSetBytes
    );
    assert_eq!(sampler.sample().unwrap(), 456);
}

#[test]
fn absent_ex2_field_is_unsupported_and_never_falls_back() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(10, None))]);
    let mut sampler = WindowsPrivateWorkingSetSampler::with_query(42, query).unwrap();

    assert!(matches!(
        sampler.sample(),
        Err(SamplerError::Unsupported {
            metric: MemoryMetric::WindowsPrivateWorkingSetBytes,
            ..
        })
    ));
}

#[test]
fn changed_process_identity_is_rejected_before_returning_memory() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(11, Some(456)))]);
    let mut sampler = WindowsPrivateWorkingSetSampler::with_query(42, query).unwrap();

    assert_eq!(
        sampler.sample(),
        Err(SamplerError::IdentityMismatch { pid: 42 })
    );
}

#[test]
fn native_query_errors_remain_typed() {
    let access_denied = SamplerError::AccessDenied { pid: 42 };
    let query = FakeQuery::new([Err(access_denied.clone())]);

    assert!(matches!(
        WindowsPrivateWorkingSetSampler::with_query(42, query),
        Err(error) if error == access_denied
    ));

    let missing = SamplerError::ProcessNotFound { pid: 42 };
    let query = FakeQuery::new([Err(missing.clone())]);
    assert!(matches!(
        WindowsPrivateWorkingSetSampler::with_query(42, query),
        Err(error) if error == missing
    ));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn windows_sampler_is_explicitly_unsupported_off_windows() {
    assert!(matches!(
        WindowsPrivateWorkingSetSampler::new(std::process::id()),
        Err(SamplerError::Unsupported {
            metric: MemoryMetric::WindowsPrivateWorkingSetBytes,
            ..
        })
    ));
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "native live-child sampler probe"]
fn live_child_reports_nonzero_private_working_set() {
    let mut sampler = WindowsPrivateWorkingSetSampler::new(std::process::id()).unwrap();

    assert_eq!(
        sampler.metric(),
        MemoryMetric::WindowsPrivateWorkingSetBytes
    );
    assert!(sampler.sample().unwrap() > 0);
}
