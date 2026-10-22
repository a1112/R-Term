use std::time::{Duration, Instant};

use rterm_runtime::{
    BatchAdmission, BatchPolicy, BatchWindow, CoalesceLatest, DrainCompletion, LatestSlot,
    PublishAction,
};

#[test]
fn byte_item_and_time_boundaries_are_exact() {
    let policy = BatchPolicy::try_new(16, 3, Duration::from_millis(3)).expect("valid policy");
    let start = Instant::now();
    let mut batch = BatchWindow::new(policy);

    assert_eq!(batch.try_push(start, 8), BatchAdmission::Accepted);
    assert_eq!(batch.try_push(start, 8), BatchAdmission::AcceptedAndFull);
    assert_eq!(batch.bytes(), 16);
    assert_eq!(batch.items(), 2);
    assert_eq!(batch.try_push(start, 1), BatchAdmission::Rejected);

    batch.reset();
    assert_eq!(batch.try_push(start, 1), BatchAdmission::Accepted);
    assert_eq!(
        batch.try_push(start + Duration::from_millis(3), 1),
        BatchAdmission::Rejected
    );
    assert_eq!(batch.items(), 1);

    batch.reset();
    assert_eq!(batch.try_push(start, 1), BatchAdmission::Accepted);
    assert_eq!(batch.try_push(start, 1), BatchAdmission::Accepted);
    assert_eq!(batch.try_push(start, 1), BatchAdmission::AcceptedAndFull);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    revision: u64,
    full_repaint: bool,
    metadata: Vec<&'static str>,
}

impl CoalesceLatest for Frame {
    fn coalesce_replaced(&mut self, replaced: Self) {
        self.full_repaint = true;
        let mut metadata = replaced.metadata;
        metadata.extend(self.metadata.iter().copied());
        self.metadata = metadata;
    }
}

#[test]
fn latest_slot_wakes_once_replaces_frames_and_requests_one_continuation() {
    let slot = LatestSlot::new();
    assert_eq!(
        slot.publish(Frame {
            revision: 1,
            full_repaint: false,
            metadata: vec!["old"],
        }),
        PublishAction::Wake
    );
    assert_eq!(
        slot.publish(Frame {
            revision: 2,
            full_repaint: false,
            metadata: vec!["new"],
        }),
        PublishAction::Coalesced
    );

    let frame = slot.take().expect("latest frame");
    assert_eq!(frame.revision, 2);
    assert!(frame.full_repaint);
    assert_eq!(frame.metadata, vec!["old", "new"]);
    assert_eq!(slot.complete_wake(false), DrainCompletion::Idle);

    assert_eq!(
        slot.publish(Frame {
            revision: 3,
            full_repaint: false,
            metadata: Vec::new(),
        }),
        PublishAction::Wake
    );
    let _ = slot.take().expect("frame under active wake");
    assert_eq!(
        slot.publish(Frame {
            revision: 4,
            full_repaint: false,
            metadata: Vec::new(),
        }),
        PublishAction::Coalesced
    );
    assert_eq!(slot.complete_wake(false), DrainCompletion::Continue);
    assert_eq!(slot.complete_wake(false), DrainCompletion::Continue);
    let _ = slot.take().expect("continuation frame");
    assert_eq!(slot.complete_wake(false), DrainCompletion::Idle);

    let metrics = slot.metrics();
    assert_eq!(metrics.publications, 4);
    assert_eq!(metrics.replaced_frames, 1);
    assert_eq!(metrics.wakes, 2);
    assert_eq!(metrics.continuations, 1);
}

#[test]
fn lossless_work_forces_continuation_even_when_latest_frame_is_empty() {
    let slot = LatestSlot::<Frame>::new();
    assert_eq!(slot.complete_wake(true), DrainCompletion::Continue);
    assert_eq!(slot.complete_wake(true), DrainCompletion::Continue);
    assert_eq!(slot.complete_wake(false), DrainCompletion::Idle);
    assert_eq!(slot.metrics().continuations, 1);
}
