use std::collections::VecDeque;

use rterm_diagnostics::{
    MacosPhysFootprintSampler, MacosProcessQuery, MacosProcessSnapshot, MemoryMetric,
    MemorySampler, SamplerError,
};

#[derive(Debug)]
struct FakeQuery {
    snapshots: VecDeque<Result<MacosProcessSnapshot, SamplerError>>,
}

impl FakeQuery {
    fn new(
        snapshots: impl IntoIterator<Item = Result<MacosProcessSnapshot, SamplerError>>,
    ) -> Self {
        Self {
            snapshots: snapshots.into_iter().collect(),
        }
    }
}

impl MacosProcessQuery for FakeQuery {
    fn snapshot(&mut self, _pid: u32) -> Result<MacosProcessSnapshot, SamplerError> {
        self.snapshots
            .pop_front()
            .expect("test query snapshot was exhausted")
    }
}

fn snapshot(identity: u64, phys_footprint_bytes: Option<u64>) -> MacosProcessSnapshot {
    MacosProcessSnapshot {
        proc_start_abstime: identity,
        phys_footprint_bytes,
        resident_size_bytes: 999,
    }
}

#[test]
fn sampler_reports_phys_footprint_without_substituting_resident_size() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(10, Some(456)))]);
    let mut sampler = MacosPhysFootprintSampler::with_query(42, query).unwrap();

    assert_eq!(sampler.metric(), MemoryMetric::MacosPhysFootprintBytes);
    assert_eq!(sampler.sample().unwrap(), 456);
}

#[test]
fn absent_phys_footprint_is_unsupported_and_never_falls_back_to_rss() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(10, None))]);
    let mut sampler = MacosPhysFootprintSampler::with_query(42, query).unwrap();

    assert!(matches!(
        sampler.sample(),
        Err(SamplerError::Unsupported {
            metric: MemoryMetric::MacosPhysFootprintBytes,
            ..
        })
    ));
}

#[test]
fn changed_process_identity_is_rejected() {
    let query = FakeQuery::new([Ok(snapshot(10, Some(123))), Ok(snapshot(11, Some(456)))]);
    let mut sampler = MacosPhysFootprintSampler::with_query(42, query).unwrap();

    assert_eq!(
        sampler.sample(),
        Err(SamplerError::IdentityMismatch { pid: 42 })
    );
}

#[test]
fn permission_and_missing_process_errors_remain_typed() {
    for error in [
        SamplerError::AccessDenied { pid: 42 },
        SamplerError::ProcessNotFound { pid: 42 },
    ] {
        let query = FakeQuery::new([Err(error.clone())]);
        assert!(matches!(
            MacosPhysFootprintSampler::with_query(42, query),
            Err(observed) if observed == error
        ));
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn macos_sampler_is_explicitly_unsupported_off_macos() {
    assert!(matches!(
        MacosPhysFootprintSampler::new(std::process::id()),
        Err(SamplerError::Unsupported {
            metric: MemoryMetric::MacosPhysFootprintBytes,
            ..
        })
    ));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "native live-child sampler probe"]
fn live_child_reports_nonzero_phys_footprint() {
    let mut sampler = MacosPhysFootprintSampler::new(std::process::id()).unwrap();

    assert_eq!(sampler.metric(), MemoryMetric::MacosPhysFootprintBytes);
    assert!(sampler.sample().unwrap() > 0);
}
