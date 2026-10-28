use crate::*;
use serde_json::json;
pub(crate) fn code_occurrence_identity(
    input: &SourceInputId,
    unit: Option<&SourceUnitKey>,
    span: &SourceSpan,
) -> EvidenceResult<OccurrenceId> {
    let SourceRef::RepositoryFile {
        repository,
        revision,
        path,
        content_identity,
    } = &span.source
    else {
        return Err(EvidenceCode::InvalidSelector.into());
    };
    if span.byte_end < span.byte_start
        || unit.is_some_and(|unit| {
            unit.repository != *repository
                || unit.path != *path
                || unit.byte_start > span.byte_start
                || unit.byte_end < span.byte_end
        })
    {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    let revision = match revision {
        SourceRevision::Commit { commit_id, side } => {
            json!({"kind":"commit","commit_id":commit_id,"side":side})
        }
        SourceRevision::Index {
            content_identity,
            side,
        } => json!({"kind":"index","content_identity":content_identity,"side":side}),
        SourceRevision::Worktree { side, .. } => {
            json!({"kind":"worktree","content_identity":content_identity,"side":side})
        }
    };
    let value = json!({"repository":repository,"revision":revision,"path":path,"unit":unit.map(|unit|(unit.byte_start,unit.byte_end)),"match":(span.byte_start,span.byte_end)});
    let bytes = serde_json::to_vec(&value).map_err(|_| EvidenceCode::InvalidSelector)?;
    OccurrenceId::new(format!("{}:{}", input.as_str(), content_digest(&bytes)))
}
