use crate::*;
use std::collections::BTreeSet;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeTextPart {
    Match,
    Hunk { hunk_id: HunkId, side: SourceSide },
    UnitText { piece_id: PieceId },
    UnitSignature { piece_id: PieceId },
    Import { position: usize },
    Hint { piece_id: PieceId },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRedactionRange {
    pub part: CodeTextPart,
    pub byte_start: usize,
    pub byte_end: usize,
}
pub struct ProjectedCodeContext {
    pub context: CandidateContext,
    pub privacy: PrivacyProjection,
    pub match_is_private: bool,
    pub redaction_ranges: Vec<CodeRedactionRange>,
}
pub fn project_code_context(
    context: &CandidateContext,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<ProjectedCodeContext> {
    let mut context = context.clone();
    let mut privacy = PrivacyProjection::default();
    let mut redaction_ranges = Vec::new();
    let (matched, projection) = project_code_text(
        &context.matched.text,
        grants,
        CodeTextPart::Match,
        &mut redaction_ranges,
    )?;
    let match_is_private = !grants.contains(&PrivateKind::CredentialLike)
        && crate::record_privacy::credential_pattern()?
            .find(&context.matched.text)
            .is_some_and(|found| found.start() == 0 && found.end() == context.matched.text.len());
    record_redactions(&context.matched.span, &projection, &mut context.redactions);
    context.matched.text = matched;
    privacy.accumulate(projection);
    for change in &mut context.changes {
        for (side, excerpt) in [
            (SourceSide::Before, change.before.as_mut()),
            (SourceSide::After, change.after.as_mut()),
        ]
        .into_iter()
        .filter_map(|(side, excerpt)| excerpt.map(|excerpt| (side, excerpt)))
        {
            project_excerpt(
                excerpt,
                grants,
                &mut privacy,
                &mut context.redactions,
                CodeTextPart::Hunk {
                    hunk_id: change.hunk_id.clone(),
                    side,
                },
                &mut redaction_ranges,
            )?;
        }
    }
    for unit in &mut context.units {
        let (text, projection) = project_code_text(
            &unit.text,
            grants,
            CodeTextPart::UnitText {
                piece_id: unit.piece_id.clone(),
            },
            &mut redaction_ranges,
        )?;
        record_redactions(&unit.unit.span, &projection, &mut context.redactions);
        unit.text = text;
        privacy.accumulate(projection);
        match &mut unit.unit.kind {
            SourceUnitKind::Function { signature }
            | SourceUnitKind::Method { signature }
            | SourceUnitKind::Class { signature } => {
                let (text, projection) = project_code_text(
                    signature,
                    grants,
                    CodeTextPart::UnitSignature {
                        piece_id: unit.piece_id.clone(),
                    },
                    &mut redaction_ranges,
                )?;
                record_redactions(&unit.unit.span, &projection, &mut context.redactions);
                *signature = text;
                privacy.accumulate(projection);
            }
            _ => {}
        }
    }
    for (position, import) in context.imports.iter_mut().enumerate() {
        project_excerpt(
            import,
            grants,
            &mut privacy,
            &mut context.redactions,
            CodeTextPart::Import { position },
            &mut redaction_ranges,
        )?;
    }
    for piece in &mut context.related {
        if let RelatedPiece::Unresolved {
            piece_id,
            name_as_written,
            call_site,
            ..
        } = piece
        {
            let (text, projection) = project_code_text(
                name_as_written,
                grants,
                CodeTextPart::Hint {
                    piece_id: piece_id.clone(),
                },
                &mut redaction_ranges,
            )?;
            record_redactions(call_site, &projection, &mut context.redactions);
            *name_as_written = text;
            privacy.accumulate(projection);
        }
    }
    Ok(ProjectedCodeContext {
        context,
        privacy,
        match_is_private,
        redaction_ranges,
    })
}
fn project_excerpt(
    excerpt: &mut CodeExcerpt,
    grants: &BTreeSet<PrivateKind>,
    privacy: &mut PrivacyProjection,
    redactions: &mut Vec<CodeRedaction>,
    part: CodeTextPart,
    ranges: &mut Vec<CodeRedactionRange>,
) -> EvidenceResult<()> {
    let (text, projection) = project_code_text(&excerpt.text, grants, part, ranges)?;
    record_redactions(&excerpt.span, &projection, redactions);
    excerpt.text = text;
    privacy.accumulate(projection);
    Ok(())
}

// Code's source line anchors survive masking. Replacement ranges cover only
// the delivered marker; preserved LF/CRLF sequences retain their source lines.
fn project_code_text(
    text: &str,
    grants: &BTreeSet<PrivateKind>,
    part: CodeTextPart,
    ranges: &mut Vec<CodeRedactionRange>,
) -> EvidenceResult<(String, PrivacyProjection)> {
    if grants.contains(&PrivateKind::CredentialLike) {
        return Ok((text.into(), PrivacyProjection::default()));
    }
    let credentials = crate::record_privacy::credential_pattern()?;
    let mut rendered = String::with_capacity(text.len());
    let mut previous_end = 0;
    let mut replaced = 0;
    for found in credentials.find_iter(text) {
        rendered.push_str(&text[previous_end..found.start()]);
        let byte_start = rendered.len();
        rendered.push_str("[redacted credential]");
        ranges.push(CodeRedactionRange {
            part: part.clone(),
            byte_start,
            byte_end: rendered.len(),
        });
        rendered.extend(
            found
                .as_str()
                .chars()
                .filter(|character| matches!(character, '\r' | '\n')),
        );
        previous_end = found.end();
        replaced += 1;
    }
    rendered.push_str(&text[previous_end..]);
    let mut privacy = PrivacyProjection::default();
    if replaced > 0 {
        privacy
            .left_out
            .insert(PrivateKind::CredentialLike, replaced);
    }
    Ok((rendered, privacy))
}

fn record_redactions(
    span: &SourceSpan,
    privacy: &PrivacyProjection,
    redactions: &mut Vec<CodeRedaction>,
) {
    for (private_kind, count) in &privacy.left_out {
        if let Some(existing) = redactions
            .iter_mut()
            .find(|item| item.span == *span && item.private_kind == *private_kind)
        {
            existing.count += count;
        } else {
            redactions.push(CodeRedaction {
                span: span.clone(),
                private_kind: *private_kind,
                count: *count,
            });
        }
    }
}
