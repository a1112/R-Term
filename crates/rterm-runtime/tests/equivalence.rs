use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use rterm_runtime::{RuntimeBuffers, RuntimeEffectRef, TerminalRuntime};
use rterm_terminal::Terminal;
use rterm_types::{DamageRegion, TerminalSize};

const REQUIRED_TRANSCRIPTS: &[&str] = &[
    "plain_ansi_query.txt",
    "alternate_screen.txt",
    "scrollback_resize.txt",
    "mouse_ime_input.txt",
    "osc_dcs_clipboard.txt",
    "metadata.txt",
    "synchronized_shutdown.txt",
    "multi_pane_restart.txt",
    "local_exit.txt",
    "ssh_disconnect.txt",
];

#[derive(Debug)]
enum Operation {
    Feed(Vec<u8>),
    Resize(TerminalSize),
    Finish,
}

#[derive(Debug)]
enum Fixture {
    Terminal {
        size: TerminalSize,
        operations: Vec<Operation>,
    },
    Contracts(Vec<(PathBuf, String)>),
}

#[derive(Debug, PartialEq, Eq)]
struct CanonicalFeed {
    responses: Vec<Vec<u8>>,
    visible: Vec<u8>,
    damage: Vec<DamageRegion>,
    bells: u64,
    diagnostics: Vec<String>,
    clipboard_writes: Vec<String>,
    clipboard_reads: Vec<String>,
    notifications: Vec<(Option<String>, String)>,
    progress: String,
    screen_identity_changed: bool,
    snapshot: String,
}

struct EquivalenceRunner {
    legacy: TerminalRuntime,
    v2: TerminalRuntime,
    buffers: RuntimeBuffers,
    legacy_deferred_damage: Vec<DamageRegion>,
}

#[test]
fn representative_equivalence_corpus_is_complete_and_green() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transcripts");
    let missing = REQUIRED_TRANSCRIPTS
        .iter()
        .filter(|name| !root.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing transcript fixtures: {missing:?}"
    );

    for name in REQUIRED_TRANSCRIPTS {
        let path = root.join(name);
        match parse_fixture(&path) {
            Fixture::Terminal { size, operations } => {
                compare_owned_legacy_and_borrowed_v2(&path, size, &operations);
            }
            Fixture::Contracts(contracts) => verify_contracts(&path, &contracts),
        }
    }
}

fn compare_owned_legacy_and_borrowed_v2(path: &Path, size: TerminalSize, operations: &[Operation]) {
    let mut runner = EquivalenceRunner {
        legacy: TerminalRuntime::new(size),
        v2: TerminalRuntime::new(size),
        buffers: RuntimeBuffers::with_capacity(8 * 1024),
        legacy_deferred_damage: Vec::new(),
    };

    for (index, operation) in operations.iter().enumerate() {
        match operation {
            Operation::Feed(bytes) => runner.compare_feed(path, index, operation, bytes),
            Operation::Resize(size) => runner.compare_resize(path, *size),
            Operation::Finish => {
                let first = runner.v2.finish_into(&mut runner.buffers);
                let first_damage = first.damage().to_vec();
                let first_effects = first.effects().map(canonical_effect).collect::<Vec<_>>();
                let second = runner.v2.finish_into(&mut runner.buffers);
                assert!(
                    second.damage().is_empty(),
                    "finish damage was not idempotent"
                );
                assert_eq!(
                    second.effects().count(),
                    0,
                    "finish effects were not idempotent"
                );
                assert!(
                    !first_damage.is_empty() || !first_effects.is_empty(),
                    "finish fixture did not exercise shutdown output: {}",
                    path.display()
                );
            }
        }
    }
}

impl EquivalenceRunner {
    fn compare_feed(&mut self, path: &Path, index: usize, operation: &Operation, bytes: &[u8]) {
        let mut legacy_output = self.legacy.feed_pty_output_with_display(bytes);
        if legacy_output
            .damage
            .starts_with(self.legacy_deferred_damage.as_slice())
        {
            legacy_output
                .damage
                .drain(..self.legacy_deferred_damage.len());
            self.legacy_deferred_damage.clear();
        }
        let legacy_feed = CanonicalFeed {
            responses: legacy_output.responses,
            visible: legacy_output.display,
            damage: legacy_output.damage,
            bells: legacy_output.bells,
            diagnostics: legacy_output.unknown_escape_sequences,
            clipboard_writes: self.legacy.take_clipboard_texts(),
            clipboard_reads: self.legacy.take_clipboard_queries(),
            notifications: self
                .legacy
                .take_notifications()
                .into_iter()
                .map(|notification| (notification.title, notification.body))
                .collect(),
            progress: format!("{:?}", self.legacy.progress()),
            screen_identity_changed: legacy_output.screen_identity_changed,
            snapshot: canonical_terminal(self.legacy.terminal()),
        };

        let delta = self.v2.feed_into(bytes, &mut self.buffers);
        let mut clipboard_writes = Vec::new();
        let mut clipboard_reads = Vec::new();
        let mut notifications = Vec::new();
        for effect in delta.effects() {
            match effect {
                RuntimeEffectRef::ClipboardWrite { contents, .. } => {
                    clipboard_writes.push(contents.to_owned());
                }
                RuntimeEffectRef::ClipboardRead { selection } => {
                    clipboard_reads.push(selection.to_owned());
                }
                RuntimeEffectRef::Notification { title, body } => {
                    notifications.push((title.map(str::to_owned), body.to_owned()));
                }
                _ => {}
            }
        }
        let v2_feed = CanonicalFeed {
            responses: delta.responses().map(<[u8]>::to_vec).collect(),
            visible: delta.visible_bytes().to_vec(),
            damage: delta.damage().to_vec(),
            bells: delta.bell_count(),
            diagnostics: delta.diagnostics().map(str::to_owned).collect(),
            clipboard_writes,
            clipboard_reads,
            notifications,
            progress: format!("{:?}", self.v2.progress()),
            screen_identity_changed: delta.screen_identity_changed(),
            snapshot: canonical_terminal(self.v2.terminal()),
        };
        assert_eq!(
            legacy_feed,
            v2_feed,
            "legacy/V2 divergence in {} operation {index}: {operation:?}",
            path.display()
        );
    }

    fn compare_resize(&mut self, path: &Path, size: TerminalSize) {
        let previous_size = self.legacy.terminal().grid().size();
        let legacy_outcome = self.legacy.resize(size);
        let (v2_outcome, delta) = self.v2.resize_into(size, &mut self.buffers);
        assert_eq!(
            legacy_outcome,
            v2_outcome,
            "resize outcome: {}",
            path.display()
        );
        let legacy_host_damage = if previous_size == size {
            Vec::new()
        } else {
            vec![DamageRegion::new(0, 0, size.columns, size.rows)]
        };
        assert_eq!(
            legacy_host_damage,
            delta.damage(),
            "resize damage: {}",
            path.display()
        );
        self.legacy_deferred_damage.clone_from(&legacy_host_damage);
        assert_eq!(
            canonical_terminal(self.legacy.terminal()),
            canonical_terminal(self.v2.terminal()),
            "resize snapshot: {}",
            path.display()
        );
    }
}

fn canonical_effect(effect: RuntimeEffectRef<'_>) -> String {
    match effect {
        RuntimeEffectRef::ConsoleWrite(bytes) => format!("console:{}", encode_hex(bytes)),
        RuntimeEffectRef::TransportWrite(bytes) => format!("transport:{}", encode_hex(bytes)),
        RuntimeEffectRef::ModeChange(change) => format!("mode:{change:?}"),
        RuntimeEffectRef::Bell { count } => format!("bell:{count}"),
        RuntimeEffectRef::ClipboardWrite {
            selection,
            contents,
        } => {
            format!("clipboard-write:{selection:?}:{contents}")
        }
        RuntimeEffectRef::ClipboardRead { selection } => format!("clipboard-read:{selection}"),
        RuntimeEffectRef::Notification { title, body } => {
            format!("notification:{title:?}:{body}")
        }
        RuntimeEffectRef::Diagnostic { message } => format!("diagnostic:{message}"),
    }
}

fn canonical_terminal(terminal: &Terminal) -> String {
    let mut output = String::new();
    let grid = terminal.grid();
    let size = grid.size();
    let mut user_vars = terminal.user_vars().iter().collect::<Vec<_>>();
    user_vars.sort_unstable_by(|left, right| left.0.cmp(right.0));
    writeln!(
        output,
        "size={}x{};cursor={:?};seq={};identity={};title={:?};cwd={:?};badge={:?};vars={user_vars:?}",
        size.columns,
        size.rows,
        terminal.cursor(),
        terminal.current_seqno(),
        terminal.screen_identity_generation(),
        terminal.title(),
        terminal.current_working_dir(),
        terminal.badge_format(),
    )
    .unwrap();
    for (index, line) in terminal.scrollback().iter().enumerate() {
        writeln!(
            output,
            "history[{index}]:seq={};wrapped={};cells={:?};overflow={:?}",
            line.last_change_seqno(),
            line.is_wrapped(),
            line.cells(),
            line.reflow_overflow(),
        )
        .unwrap();
    }
    for row in 0..size.rows {
        let line = grid.row(row).expect("row within terminal size");
        writeln!(
            output,
            "row[{row}]:seq={};wrapped={};cells={:?};overflow={:?}",
            line.last_change_seqno(),
            line.is_wrapped(),
            line.cells(),
            line.reflow_overflow(),
        )
        .unwrap();
    }
    output
}

fn verify_contracts(fixture: &Path, contracts: &[(PathBuf, String)]) {
    assert!(
        !contracts.is_empty(),
        "empty contract fixture: {}",
        fixture.display()
    );
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("runtime crate is inside workspace");
    for (relative, test_name) in contracts {
        let source_path = workspace.join(relative);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("read contract {}: {error}", source_path.display()));
        let declaration = format!("fn {test_name}(");
        assert!(
            source.contains(&declaration),
            "{} references missing contract {} in {}",
            fixture.display(),
            test_name,
            source_path.display()
        );
    }
}

fn parse_fixture(path: &Path) -> Fixture {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read fixture {}: {error}", path.display()));
    let mut kind = None;
    let mut size = None;
    let mut operations = Vec::new();
    let mut contracts = Vec::new();
    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "kind" => kind = Some(value),
            "size" => {
                let (columns, rows) = value.split_once('x').expect("size uses CxR");
                size = Some(TerminalSize::new(
                    columns.parse().expect("numeric columns"),
                    rows.parse().expect("numeric rows"),
                ));
            }
            "feed" => operations.push(Operation::Feed(decode_hex(value))),
            "resize" => {
                let (columns, rows) = value.split_once('x').expect("resize uses CxR");
                operations.push(Operation::Resize(TerminalSize::new(
                    columns.parse().expect("numeric columns"),
                    rows.parse().expect("numeric rows"),
                )));
            }
            "finish" => operations.push(Operation::Finish),
            "contract" => {
                let (source, test) = value.split_once('|').expect("contract uses path|test");
                contracts.push((PathBuf::from(source), test.to_owned()));
            }
            _ => panic!(
                "{}:{} unknown fixture key {key}",
                path.display(),
                line_index + 1
            ),
        }
    }
    match kind {
        Some("terminal") => Fixture::Terminal {
            size: size.expect("terminal fixture size"),
            operations,
        },
        Some("contracts") => Fixture::Contracts(contracts),
        _ => panic!("{} has invalid or missing kind", path.display()),
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2), "odd hex fixture payload");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("ASCII hex");
            u8::from_str_radix(text, 16).expect("valid hex")
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing into a String is infallible");
        output
    })
}
