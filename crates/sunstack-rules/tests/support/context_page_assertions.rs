#![allow(dead_code)]
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::*;
use sunstack_rules::{
    CandidateContextView, ContextPieceDisposition, RelatedBasis, RelatednessLabel,
    UncertaintyReason,
};

fn resolve<'piece>(
    display: &str,
    pieces: impl Iterator<Item = &'piece PieceId>,
) -> Result<&'piece PieceId, &'static str> {
    let mut candidates = pieces.filter(|id| {
        display
            .strip_prefix('…')
            .map_or(id.as_str() == display, |suffix| {
                id.as_str().ends_with(suffix)
            })
    });
    let piece = candidates.next().ok_or("dangling or ambiguous piece")?;
    if candidates.next().is_some() {
        return Err("dangling or ambiguous piece");
    }
    Ok(piece)
}
fn native_name(value: &impl Serialize) -> Result<String, &'static str> {
    serde_json::to_value(value)
        .map_err(|_| "invalid native enum")?
        .as_str()
        .map(str::to_owned)
        .ok_or("invalid native enum")
}
fn locator(span: &SourceSpan) -> String {
    let source = match &span.source {
        SourceRef::RepositoryFile { path, revision, .. } => {
            let (SourceRevision::Commit { side, .. }
            | SourceRevision::Index { side, .. }
            | SourceRevision::Worktree { side, .. }) = revision;
            format!(
                "{} [{}]",
                path.display(),
                if *side == SourceSide::Before {
                    "before"
                } else {
                    "after"
                }
            )
        }
        SourceRef::Document { path, .. } | SourceRef::Transcript { path, .. } => {
            path.display().to_string()
        }
        SourceRef::Payload { input, .. } => format!("payload {}", input.as_str()),
    };
    format!(
        "{source}:{}:{}-{}:{}",
        span.start_line, span.start_column, span.end_line, span.end_column
    )
}
fn unit_locator(view: &CandidateContextView, key: &SourceUnitKey) -> Result<String, &'static str> {
    view.context
        .units
        .iter()
        .find(|unit| unit.unit.key == *key)
        .map(|unit| locator(&unit.unit.span))
        .ok_or("missing native unit locator")
}
fn piece_locator(view: &CandidateContextView, id: &PieceId) -> Result<String, &'static str> {
    if let Some(unit) = view.context.units.iter().find(|unit| unit.piece_id == *id) {
        return Ok(locator(&unit.unit.span));
    }
    for relation in &view.context.related {
        match relation {
            RelatedPiece::Resolved {
                piece_id,
                seed,
                unit,
                connection,
                facts,
                ..
            } if piece_id == id => {
                let kinds = facts
                    .iter()
                    .map(|fact| {
                        serde_json::to_value(fact).map_err(|_| "invalid native fact")?["kind"]
                            .as_str()
                            .map(str::to_owned)
                            .ok_or("invalid native fact")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(format!(
                    "{}: {} -> {}; facts: {}",
                    native_name(connection)?,
                    unit_locator(view, seed)?,
                    unit_locator(view, unit)?,
                    kinds.join(", ")
                ));
            }
            RelatedPiece::Unresolved {
                piece_id,
                connection,
                name_as_written,
                call_site,
                reason,
                ..
            } if piece_id == id => {
                return Ok(format!(
                    "{}: {name_as_written}; call_site {}; {} (source not found)",
                    native_name(connection)?,
                    locator(call_site),
                    native_name(reason)?
                ));
            }
            _ => {}
        }
    }
    if id.as_str() == format!("match:{}", view.context.context_id.as_str()) {
        return Ok(locator(&view.context.matched.span));
    }
    for hunk in &view.context.changes {
        for (side, excerpt) in [("before", &hunk.before), ("after", &hunk.after)] {
            if id.as_str() == format!("hunk:{}:{side}", hunk.hunk_id.as_str())
                && let Some(excerpt) = excerpt
            {
                return Ok(locator(&excerpt.span));
            }
        }
    }
    for excerpt in &view.context.imports {
        let span = serde_json::to_vec(&excerpt.span).map_err(|_| "invalid native import")?;
        if id.as_str()
            == format!(
                "import:{}:{}",
                view.context.context_id.as_str(),
                content_digest(&span)
            )
        {
            return Ok(locator(&excerpt.span));
        }
    }
    Err("piece has no native locator")
}
fn check_label(
    view: &CandidateContextView,
    id: &PieceId,
    rendered: &str,
) -> Result<(), &'static str> {
    let native = view.labels.get(id).ok_or("unknown bundle label")?;
    let expected = match native {
        RelatednessLabel::Related { basis } => match basis {
            RelatedBasis::InputUnit { unit } => {
                format!("related — input unit {}", unit_locator(view, unit)?)
            }
            RelatedBasis::InputSpan { span } => format!("related — input span {}", locator(span)),
            RelatedBasis::Judged { answer } => format!("related — judged by {}", answer.0.as_str()),
        },
        RelatednessLabel::Uncertain { reasons } => format!(
            "uncertain — {}",
            reasons
                .iter()
                .map(|reason| match reason {
                    UncertaintyReason::Graph { reason } => native_name(reason),
                    UncertaintyReason::Contradicted { reason } => native_name(reason),
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        RelatednessLabel::Unrelated { judged_by } => {
            format!("unrelated — judged by {}", judged_by.0.as_str())
        }
        RelatednessLabel::ProbablyRelated { fact } => {
            let tail = rendered
                .strip_prefix("probably_related — graph fact ")
                .ok_or("wrong graph label")?;
            let (display, position) = tail.split_once('#').ok_or("missing graph position")?;
            let piece = resolve(display, view.labels.keys())?;
            if *piece != fact.piece_id || position.parse::<usize>().ok() != Some(fact.position) {
                return Err("wrong graph basis");
            }
            let relation = view.context.related.iter().find(|relation| matches!(relation, RelatedPiece::Resolved { piece_id, .. } if piece_id == piece)).ok_or("missing graph source")?;
            if !matches!(relation, RelatedPiece::Resolved { facts, .. } if facts.get(fact.position).is_some())
            {
                return Err("missing native fact");
            }
            return Ok(());
        }
    };
    if rendered != expected {
        return Err("rendered label or basis differs from native bundle");
    }
    Ok(())
}

pub(super) fn check_bundle_pages(view: &CandidateContextView) -> Result<(), &'static str> {
    let expected = view.labels.keys().collect::<BTreeSet<_>>();
    let covered = view
        .coverage
        .iter()
        .map(|piece| &piece.piece_id)
        .collect::<BTreeSet<_>>();
    if expected != covered || view.coverage.len() != covered.len() {
        return Err("bundle coverage mismatch");
    }
    let mut placed = BTreeMap::<&PieceId, Vec<usize>>::new();
    for page in &view.pages {
        let mut shown = Vec::new();
        let mut current = None;
        let mut labels = BTreeMap::<&PieceId, Vec<&str>>::new();
        let mut anchors = BTreeMap::<&PieceId, Vec<&str>>::new();
        for line in page.text.lines() {
            if let Some(start) = line
                .strip_prefix("piece [")
                .or_else(|| line.strip_prefix("match anchor ["))
            {
                let (display, _) = start.split_once(']').ok_or("invalid piece marker")?;
                let piece = resolve(display, page.holds.iter())?;
                if !expected.contains(&piece) {
                    return Err("unknown bundle piece");
                }
                shown.push(piece);
                placed.entry(piece).or_default().push(page.number);
                current = Some(piece);
            } else if let Some(label) = line.strip_prefix("label: ") {
                labels
                    .entry(current.ok_or("orphan label")?)
                    .or_default()
                    .push(label);
            } else if let Some(anchor) = line.strip_prefix("anchor: ") {
                anchors
                    .entry(current.ok_or("orphan locator")?)
                    .or_default()
                    .push(anchor);
            }
        }
        if shown.iter().copied().collect::<BTreeSet<_>>() != page.holds.iter().collect()
            || shown.len() != page.holds.len()
        {
            return Err("orphan or duplicate page piece");
        }
        for piece in shown {
            let [label] = labels.get(piece).map(Vec::as_slice).unwrap_or_default() else {
                return Err("missing or duplicate label");
            };
            let [anchor] = anchors.get(piece).map(Vec::as_slice).unwrap_or_default() else {
                return Err("missing or duplicate locator");
            };
            if *anchor != piece_locator(view, piece)? {
                return Err("wrong native path side or line range");
            }
            check_label(view, piece, label)?;
        }
    }
    for entry in &view.coverage {
        let pages = placed
            .get(&entry.piece_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        match entry.disposition {
            ContextPieceDisposition::Placed { page }
            | ContextPieceDisposition::Truncated { page, .. }
                if pages == [page] => {}
            ContextPieceDisposition::Excluded { .. }
            | ContextPieceDisposition::NeverFound { .. }
                if pages.len() <= 1 => {}
            _ => return Err("piece placement mismatch"),
        }
    }
    Ok(())
}
