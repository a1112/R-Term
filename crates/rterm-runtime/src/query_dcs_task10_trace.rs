use super::*;

pub(super) fn trace_decrqss(bytes: &[u8], result: Option<DecrqssRequest>) {
    let result = result.map_or_else(
        || "kind=decrqss;result=none".to_owned(),
        |request| {
            format!(
                "kind=decrqss;result=some:{}:{}",
                decrqss_kind_tag(request.kind),
                terminator_tag(request.terminator)
            )
        },
    );
    trace_pure("dcs.parse_decrqss", bytes, result.as_bytes());
}

pub(super) fn trace_xtgettcap(bytes: &[u8], result: Option<&XtGetTcapRequest>) {
    let result = result.map_or_else(
        || "kind=xtgettcap;result=none".to_owned(),
        |request| {
            let names = request
                .names
                .iter()
                .map(|name| {
                    format!(
                        "{}:{}",
                        crate::fixture_trace::encode_hex(&name.encoded),
                        name.decoded.as_ref().map_or_else(
                            || "none".to_owned(),
                            |decoded| format!("some:{}", crate::fixture_trace::encode_hex(decoded))
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "kind=xtgettcap;result=some:{}:{names}",
                terminator_tag(request.terminator)
            )
        },
    );
    trace_pure("dcs.parse_xtgettcap", bytes, result.as_bytes());
}

pub(super) fn trace_reserved(bytes: &[u8], result: bool) {
    let result = format!("kind=reserved;result={}", u8::from(result));
    trace_pure("dcs.is_reserved", bytes, result.as_bytes());
}

fn trace_pure(operation: &'static str, bytes: &[u8], result: &[u8]) {
    let state = b"kind=pure;pending=";
    let object = crate::fixture_trace::new_object("dcs", operation, bytes, result, state);
    crate::fixture_trace::finish_object("dcs", object, b"", state, state);
}

fn decrqss_kind_tag(kind: DecrqssKind) -> &'static str {
    match kind {
        DecrqssKind::Sgr => "sgr",
        DecrqssKind::CursorShape => "cursor-shape",
        DecrqssKind::ScrollRegion => "scroll-region",
        DecrqssKind::ConformanceLevel => "conformance-level",
        DecrqssKind::LeftRightMargins => "left-right-margins",
        DecrqssKind::Unknown => "unknown",
    }
}

fn terminator_tag(terminator: DcsTerminator) -> &'static str {
    match terminator {
        DcsTerminator::SevenBit => "7-bit",
        DcsTerminator::EightBit => "8-bit",
    }
}
