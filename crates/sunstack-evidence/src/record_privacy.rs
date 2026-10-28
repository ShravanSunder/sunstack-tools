use crate::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct PrivacyProjection {
    pub left_out: BTreeMap<PrivateKind, usize>,
}
impl PrivacyProjection {
    pub(crate) fn accumulate(&mut self, other: Self) {
        for (kind, count) in other.left_out {
            *self.left_out.entry(kind).or_default() += count;
        }
    }
}

pub(crate) fn retain_private_blocks(
    blocks: Vec<PositionedBlock>,
    grants: &BTreeSet<PrivateKind>,
    privacy: &mut PrivacyProjection,
) -> Vec<(usize, PositionedBlock)> {
    blocks
        .into_iter()
        .enumerate()
        .filter(|(_, block)| {
            if let Some(kind) = block.private_kind.filter(|kind| !grants.contains(kind)) {
                *privacy.left_out.entry(kind).or_default() += 1;
                false
            } else {
                true
            }
        })
        .collect()
}
#[derive(Clone)]
pub struct ProjectedRecord {
    pub record: Option<PositionedRecord>,
    pub privacy: PrivacyProjection,
}
pub struct ProjectedCoverageGaps {
    pub gaps: Vec<CoverageGap>,
    pub privacy: PrivacyProjection,
}
pub fn project_coverage_gaps(
    gaps: &[CoverageGap],
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<ProjectedCoverageGaps> {
    let mut value = serde_json::to_value(gaps).map_err(|_| EvidenceCode::InvalidSelector)?;
    let mut privacy = PrivacyProjection::default();
    project_json_text(&mut value, grants, &mut privacy)?;
    let gaps = serde_json::from_value(value).map_err(|_| EvidenceCode::InvalidSelector)?;
    Ok(ProjectedCoverageGaps { gaps, privacy })
}

pub fn match_is_private(
    record: &PositionedRecord,
    occurrence: &SourceOccurrence,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<bool> {
    let SourceOccurrence::Record { field, span, .. } = occurrence else {
        return Err(EvidenceCode::InvalidSelector.into());
    };
    match &record.body {
        PositionedBody::Private { private_kind, .. } if !grants.contains(private_kind) => {
            return Ok(true);
        }
        PositionedBody::Message { blocks } => {
            if let Some(index) = field
                .as_str()
                .strip_prefix("/body/blocks/")
                .and_then(|rest| rest.split('/').next())
                .and_then(|index| index.parse::<usize>().ok())
                && let Some(block) = blocks.get(index)
                && block
                    .private_kind
                    .is_some_and(|kind| !grants.contains(&kind))
            {
                return Ok(true);
            }
        }
        PositionedBody::Custom { field_privacy, .. } => {
            if let Some(name) = field
                .as_str()
                .strip_prefix("/body/fields/")
                .and_then(|tail| tail.split('/').next())
            {
                let name = name.replace("~1", "/").replace("~0", "~");
                if let Some(classification) = field_privacy.get(&name) {
                    return Ok(!classification.readable
                        || classification
                            .private_kind
                            .is_some_and(|kind| !grants.contains(&kind)));
                }
            }
        }
        PositionedBody::Private { .. }
        | PositionedBody::ToolCall { .. }
        | PositionedBody::ToolResult { .. } => {}
    }
    if !grants.contains(&PrivateKind::CredentialLike) {
        let value = serde_json::to_value(record).map_err(|_| EvidenceCode::InvalidSelector)?;
        if let Some(text) = value
            .pointer(field.as_str())
            .and_then(serde_json::Value::as_str)
        {
            return Ok(credential_pattern()?.find_iter(text).any(|matched| {
                matched.start() < span.byte_end && span.byte_start < matched.end()
            }));
        }
    }
    Ok(false)
}

pub fn project_record(
    record: &PositionedRecord,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<ProjectedRecord> {
    let mut privacy = PrivacyProjection::default();
    let mut record = record.clone();
    match &mut record.body {
        PositionedBody::Private { private_kind, .. } if !grants.contains(private_kind) => {
            privacy.left_out.insert(*private_kind, 1);
            return Ok(ProjectedRecord {
                record: None,
                privacy,
            });
        }
        PositionedBody::Message { blocks } => {
            let mut kept = Vec::new();
            let mut positions = BTreeMap::new();
            for (old_index, block) in
                retain_private_blocks(std::mem::take(blocks), grants, &mut privacy)
            {
                let old_path = FieldPath::new(format!("/body/blocks/{old_index}"))?;
                if let Some(position) = record.field_positions.get(&old_path) {
                    positions.insert(
                        FieldPath::new(format!("/body/blocks/{}", kept.len()))?,
                        *position,
                    );
                }
                kept.push(block);
            }
            *blocks = kept;
            record.field_positions = positions;
            if blocks.is_empty() {
                return Ok(ProjectedRecord {
                    record: None,
                    privacy,
                });
            }
        }
        PositionedBody::Custom {
            fields,
            field_privacy,
        } => {
            let classified = !field_privacy.is_empty();
            retain_mapped_fields(fields, field_privacy, grants, &mut privacy);
            if classified && fields.is_empty() {
                return Ok(ProjectedRecord {
                    record: None,
                    privacy,
                });
            }
        }
        PositionedBody::Private { .. }
        | PositionedBody::ToolCall { .. }
        | PositionedBody::ToolResult { .. } => {}
    }
    let mut body = serde_json::to_value(&record.body).map_err(|_| EvidenceCode::InvalidSelector)?;
    project_json_text(&mut body, grants, &mut privacy)?;
    record.body = serde_json::from_value(body).map_err(|_| EvidenceCode::InvalidSelector)?;
    Ok(ProjectedRecord {
        record: Some(record),
        privacy,
    })
}

pub fn project_text(
    text: &str,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<(String, PrivacyProjection)> {
    if grants.contains(&PrivateKind::CredentialLike) {
        return Ok((text.into(), PrivacyProjection::default()));
    }
    let credentials = credential_pattern()?;
    let count = credentials.find_iter(text).count();
    let mut privacy = PrivacyProjection::default();
    if count > 0 {
        privacy.left_out.insert(PrivateKind::CredentialLike, count);
    }
    Ok((
        credentials
            .replace_all(text, "[redacted credential]")
            .into_owned(),
        privacy,
    ))
}

pub(crate) fn credential_pattern() -> EvidenceResult<regex::Regex> {
    regex::Regex::new(r#"(?i)(?:sk-[a-z0-9_-]{12,}|gh[pousr]_[a-z0-9]{20,}|AKIA[A-Z0-9]{16}|(?:api[_-]?key|password|secret|access[_-]?token)\s*[:=]\s*["']?[a-z0-9_./+\-]{12,})"#)
        .map_err(|_| EvidenceCode::InvalidSelector.into())
}

pub(crate) fn project_json_text(
    value: &mut serde_json::Value,
    grants: &BTreeSet<PrivateKind>,
    privacy: &mut PrivacyProjection,
) -> EvidenceResult<()> {
    match value {
        serde_json::Value::String(text) => {
            let (projected, report) = project_text(text, grants)?;
            *text = projected;
            for (kind, count) in report.left_out {
                *privacy.left_out.entry(kind).or_default() += count;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                project_json_text(item, grants, privacy)?;
            }
        }
        serde_json::Value::Object(fields) => {
            for item in fields.values_mut() {
                project_json_text(item, grants, privacy)?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
    Ok(())
}

pub fn project_structured_json(
    value: &StructuredJson,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<(StructuredJson, PrivacyProjection)> {
    let mut value = serde_json::to_value(value).map_err(|_| EvidenceCode::InvalidSelector)?;
    let mut privacy = PrivacyProjection::default();
    project_json_text(&mut value, grants, &mut privacy)?;
    let value = serde_json::from_value(value).map_err(|_| EvidenceCode::InvalidSelector)?;
    Ok((value, privacy))
}

pub(crate) fn retain_mapped_fields(
    fields: &mut BTreeMap<String, StructuredJson>,
    classifications: &mut BTreeMap<String, MappedFieldPrivacy>,
    grants: &BTreeSet<PrivateKind>,
    privacy: &mut PrivacyProjection,
) {
    classifications.retain(|name, classification| {
        let denied = !classification.readable
            || classification
                .private_kind
                .is_some_and(|kind| !grants.contains(&kind));
        if denied {
            if let Some(kind) = classification.private_kind {
                *privacy.left_out.entry(kind).or_default() += 1;
            }
            fields.remove(name);
        }
        !denied
    });
}
