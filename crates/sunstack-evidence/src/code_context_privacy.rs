use crate::*;
use std::collections::BTreeSet;
pub struct ProjectedCodeContext {
    pub context: CandidateContext,
    pub privacy: PrivacyProjection,
    pub match_is_private: bool,
}
pub fn project_code_context(
    context: &CandidateContext,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<ProjectedCodeContext> {
    let mut context = context.clone();
    let mut privacy = PrivacyProjection::default();
    let (matched, projection) = project_text(&context.matched.text, grants)?;
    let match_is_private = !grants.contains(&PrivateKind::CredentialLike)
        && crate::record_privacy::credential_pattern()?
            .find(&context.matched.text)
            .is_some_and(|found| found.start() == 0 && found.end() == context.matched.text.len());
    record_redactions(&context.matched.span, &projection, &mut context.redactions);
    context.matched.text = matched;
    privacy.accumulate(projection);
    for change in &mut context.changes {
        for excerpt in [change.before.as_mut(), change.after.as_mut()]
            .into_iter()
            .flatten()
        {
            project_excerpt(excerpt, grants, &mut privacy, &mut context.redactions)?;
        }
    }
    for unit in &mut context.units {
        let (text, projection) = project_text(&unit.text, grants)?;
        record_redactions(&unit.unit.span, &projection, &mut context.redactions);
        unit.text = text;
        privacy.accumulate(projection);
        match &mut unit.unit.kind {
            SourceUnitKind::Function { signature }
            | SourceUnitKind::Method { signature }
            | SourceUnitKind::Class { signature } => {
                let (text, projection) = project_text(signature, grants)?;
                record_redactions(&unit.unit.span, &projection, &mut context.redactions);
                *signature = text;
                privacy.accumulate(projection);
            }
            _ => {}
        }
    }
    for import in &mut context.imports {
        project_excerpt(import, grants, &mut privacy, &mut context.redactions)?;
    }
    for piece in &mut context.related {
        if let RelatedPiece::Unresolved {
            name_as_written,
            call_site,
            ..
        } = piece
        {
            let (text, projection) = project_text(name_as_written, grants)?;
            record_redactions(call_site, &projection, &mut context.redactions);
            *name_as_written = text;
            privacy.accumulate(projection);
        }
    }
    Ok(ProjectedCodeContext {
        context,
        privacy,
        match_is_private,
    })
}
fn project_excerpt(
    excerpt: &mut CodeExcerpt,
    grants: &BTreeSet<PrivateKind>,
    privacy: &mut PrivacyProjection,
    redactions: &mut Vec<CodeRedaction>,
) -> EvidenceResult<()> {
    let (text, projection) = project_text(&excerpt.text, grants)?;
    record_redactions(&excerpt.span, &projection, redactions);
    excerpt.text = text;
    privacy.accumulate(projection);
    Ok(())
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
