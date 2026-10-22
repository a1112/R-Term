use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    fmt::Write as _,
    panic::{AssertUnwindSafe, catch_unwind},
};

use sha2::{Digest, Sha256};

thread_local! {
    static ACTIVE_CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct Blob {
    kind: &'static str,
    sha256: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct Action {
    sequence: usize,
    layer: &'static str,
    object: u64,
    operation: &'static str,
    arguments: usize,
    result: usize,
    pre_state: Option<usize>,
    post_state: usize,
}

#[derive(Debug)]
struct FinalObject {
    layer: &'static str,
    object: u64,
    pending: usize,
    state: usize,
    snapshot: usize,
}

#[derive(Clone, Copy)]
struct ActionInput<'a> {
    layer: &'static str,
    object: u64,
    operation: &'static str,
    arguments: &'a [u8],
    result: &'a [u8],
    pre_state: Option<&'a [u8]>,
    post_state: &'a [u8],
}

#[derive(Debug)]
struct Capture {
    row_id: String,
    domain: &'static str,
    next_object: u64,
    blobs: Vec<Blob>,
    blob_indexes: HashMap<String, usize>,
    actions: Vec<Action>,
    last_post: HashMap<(&'static str, u64), usize>,
    final_objects: Vec<FinalObject>,
}

impl Capture {
    fn new(row_id: &str, domain: &'static str) -> Self {
        Self {
            row_id: row_id.to_owned(),
            domain,
            next_object: 1,
            blobs: Vec::new(),
            blob_indexes: HashMap::new(),
            actions: Vec::new(),
            last_post: HashMap::new(),
            final_objects: Vec::new(),
        }
    }

    fn accepts(&self, layer: &str) -> bool {
        match self.domain {
            "runtime" => layer == "runtime",
            "runtime_filter" => layer == "runtime-filter",
            "query" => layer == "query",
            "dcs" => layer == "dcs",
            "terminal_parser" => layer == "terminal-parser",
            _ => false,
        }
    }

    fn blob(&mut self, kind: &'static str, bytes: &[u8]) -> usize {
        let sha256 = sha256_hex(bytes);
        let key = format!("{kind}:{sha256}:{}", bytes.len());
        if let Some(index) = self.blob_indexes.get(&key).copied()
            && self.blobs[index].bytes == bytes
        {
            return index;
        }
        let index = self.blobs.len();
        self.blobs.push(Blob {
            kind,
            sha256,
            bytes: bytes.to_vec(),
        });
        self.blob_indexes.insert(key, index);
        index
    }

    fn push_action(&mut self, action: ActionInput<'_>) {
        let ActionInput {
            layer,
            object,
            operation,
            arguments,
            result,
            pre_state,
            post_state,
        } = action;
        let arguments = self.blob("arguments", arguments);
        let result = self.blob("ordered-observables", result);
        let pre_state = pre_state.map(|state| self.blob("pre-state", state));
        let post_state = self.blob("post-state", post_state);
        let key = (layer, object);
        if let (Some(previous), Some(current)) = (self.last_post.get(&key).copied(), pre_state)
            && self.blobs[previous].bytes != self.blobs[current].bytes
        {
            let empty = self.blob("arguments", b"");
            let transition_result = self.blob(
                "ordered-observables",
                b"kind=private-state-transition;callbacks=;pending=",
            );
            let sequence = self.actions.len();
            self.actions.push(Action {
                sequence,
                layer,
                object,
                operation: "test_internal_state_injection",
                arguments: empty,
                result: transition_result,
                pre_state: Some(previous),
                post_state: current,
            });
        }
        let sequence = self.actions.len();
        self.actions.push(Action {
            sequence,
            layer,
            object,
            operation,
            arguments,
            result,
            pre_state,
            post_state,
        });
        self.last_post.insert(key, post_state);
    }

    fn finish_object(
        &mut self,
        layer: &'static str,
        object: u64,
        pending: &[u8],
        state: &[u8],
        snapshot: &[u8],
    ) {
        let key = (layer, object);
        let state_index = self.blob("final-state", state);
        if let Some(previous) = self.last_post.get(&key).copied()
            && self.blobs[previous].bytes != state
        {
            let empty = self.blob("arguments", b"");
            let result = self.blob(
                "ordered-observables",
                b"kind=private-state-transition;callbacks=;pending=",
            );
            let sequence = self.actions.len();
            self.actions.push(Action {
                sequence,
                layer,
                object,
                operation: "test_internal_state_injection",
                arguments: empty,
                result,
                pre_state: Some(previous),
                post_state: state_index,
            });
        }
        let pending = self.blob("final-pending", pending);
        let snapshot = self.blob("final-snapshot", snapshot);
        self.last_post.insert(key, state_index);
        self.final_objects.push(FinalObject {
            layer,
            object,
            pending,
            state: state_index,
            snapshot,
        });
    }

    fn encode(self) -> Vec<u8> {
        let mut output = String::new();
        output.push_str("schema=rssh.task10.canonical-trace/v1\n");
        writeln!(&mut output, "row_id={}", self.row_id).expect("write trace row");
        writeln!(&mut output, "domain={}", self.domain).expect("write trace domain");
        writeln!(&mut output, "init=content-addressed-state").expect("write trace init");
        writeln!(&mut output, "action_count={}", self.actions.len())
            .expect("write trace action count");
        for action in &self.actions {
            let pre = action
                .pre_state
                .map_or_else(|| "empty".to_owned(), blob_reference);
            writeln!(
                &mut output,
                "action={}|api={}|config=explicit|size=state|state={}|input={}|chunk={}|finish={}|resize={}|reset={}|parent=none|layer={}|object={}",
                action.sequence,
                action.operation,
                pre,
                blob_reference(action.arguments),
                action.sequence,
                u8::from(action.operation.contains("finish")),
                u8::from(action.operation.contains("resize")),
                u8::from(action.operation.contains("reset")),
                action.layer,
                action.object,
            )
            .expect("write trace action");
            writeln!(
                &mut output,
                "observables={}|typed={}|responses={}|effects={}|display={}|visible={}|damage={}|metadata={}|bells={}|clipboard={}|notifications={}|diagnostics={}|identity={}|callbacks={}|pending={}|snapshot={}",
                action.sequence,
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.result),
                blob_reference(action.post_state),
            )
            .expect("write trace observables");
        }
        for final_object in &self.final_objects {
            writeln!(
                &mut output,
                "final_object={}:{};pending={};state={};snapshot={}",
                final_object.layer,
                final_object.object,
                blob_reference(final_object.pending),
                blob_reference(final_object.state),
                blob_reference(final_object.snapshot),
            )
            .expect("write trace final object");
        }
        let final_pending = self.final_objects.last().map_or_else(
            || "empty".to_owned(),
            |object| blob_reference(object.pending),
        );
        let final_state = self
            .final_objects
            .last()
            .map_or_else(|| "empty".to_owned(), |object| blob_reference(object.state));
        let final_snapshot = self.final_objects.last().map_or_else(
            || "empty".to_owned(),
            |object| blob_reference(object.snapshot),
        );
        writeln!(&mut output, "final_pending={final_pending}").expect("write final pending");
        writeln!(&mut output, "final_state={final_state}").expect("write final state");
        writeln!(&mut output, "final_snapshot={final_snapshot}").expect("write final snapshot");
        for (index, blob) in self.blobs.iter().enumerate() {
            writeln!(
                &mut output,
                "blob={index}|kind={}|len={}|sha256={}|bytes={}",
                blob.kind,
                blob.bytes.len(),
                blob.sha256,
                encode_hex(&blob.bytes),
            )
            .expect("write trace blob");
        }
        output.into_bytes()
    }
}

pub(crate) fn capture(
    row_id: &str,
    domain: &'static str,
    run: impl FnOnce(),
) -> (Result<(), Box<dyn Any + Send>>, Vec<u8>) {
    ACTIVE_CAPTURE.with(|active| {
        assert!(active.borrow().is_none(), "nested fixture capture");
        *active.borrow_mut() = Some(Capture::new(row_id, domain));
    });
    let result = catch_unwind(AssertUnwindSafe(run));
    let capture =
        ACTIVE_CAPTURE.with(|active| active.borrow_mut().take().expect("active fixture capture"));
    (result, capture.encode())
}

pub(crate) fn new_object(
    layer: &'static str,
    operation: &'static str,
    arguments: &[u8],
    result: &[u8],
    state: &[u8],
) -> u64 {
    ACTIVE_CAPTURE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(capture) = active.as_mut() else {
            return 0;
        };
        if !capture.accepts(layer) {
            return 0;
        }
        let object = capture.next_object;
        capture.next_object = capture.next_object.saturating_add(1);
        capture.push_action(ActionInput {
            layer,
            object,
            operation,
            arguments,
            result,
            pre_state: None,
            post_state: state,
        });
        object
    })
}

pub(crate) fn has_object(layer: &'static str) -> bool {
    ACTIVE_CAPTURE.with(|active| {
        active.borrow().as_ref().is_some_and(|capture| {
            capture.accepts(layer)
                && capture
                    .last_post
                    .keys()
                    .any(|(object_layer, _)| *object_layer == layer)
        })
    })
}

pub(crate) fn record_action(
    layer: &'static str,
    object: u64,
    operation: &'static str,
    arguments: &[u8],
    result: &[u8],
    pre_state: &[u8],
    post_state: &[u8],
) {
    if object == 0 {
        return;
    }
    ACTIVE_CAPTURE.with(|active| {
        let mut active = active.borrow_mut();
        let capture = active.as_mut().expect("fixture object outside capture");
        capture.push_action(ActionInput {
            layer,
            object,
            operation,
            arguments,
            result,
            pre_state: Some(pre_state),
            post_state,
        });
    });
}

pub(crate) fn finish_object(
    layer: &'static str,
    object: u64,
    pending: &[u8],
    state: &[u8],
    snapshot: &[u8],
) {
    if object == 0 {
        return;
    }
    ACTIVE_CAPTURE.with(|active| {
        let mut active = active.borrow_mut();
        let capture = active.as_mut().expect("fixture object outside capture");
        capture.finish_object(layer, object, pending, state, snapshot);
    });
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("write SHA-256 to String");
    }
    encoded
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub(crate) fn encode_exact_runs(bytes: &[u8]) -> String {
    let mut encoded = format!("len={};runs=", bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        let byte = bytes[offset];
        let mut end = offset + 1;
        while end < bytes.len() && bytes[end] == byte {
            end += 1;
        }
        if offset != 0 {
            encoded.push(',');
        }
        write!(&mut encoded, "{byte:02x}*{}", end - offset).expect("write exact byte run");
        offset = end;
    }
    encoded
}

fn blob_reference(index: usize) -> String {
    format!("blob:{index}")
}
