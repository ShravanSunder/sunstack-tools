use crate::*;
use std::collections::BTreeSet;

impl ConversationSelection {
    pub fn validate(&self) -> EvidenceResult<()> {
        if self.user_messages == Inclusion::Excluded
            && self.assistant_messages == AssistantSelection::Excluded
            && matches!(self.tool_use, ToolUseSelection::Excluded)
        {
            return Err(EvidenceCode::EmptyProjection.into());
        }
        if let ToolUseSelection::Included { tools, .. } = &self.tool_use {
            match tools {
                ToolFilter::Named { names }
                    if names.is_empty()
                        || names.iter().any(String::is_empty)
                        || names.iter().collect::<BTreeSet<_>>().len() != names.len() =>
                {
                    return Err(EvidenceCode::InvalidSelector.into());
                }
                ToolFilter::Categories { categories }
                    if categories.is_empty()
                        || categories
                            .iter()
                            .map(|category| format!("{category:?}"))
                            .collect::<BTreeSet<_>>()
                            .len()
                            != categories.len() =>
                {
                    return Err(EvidenceCode::InvalidSelector.into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ConversationProjection {
    pub records: Vec<ConversationRecord>,
    pub omissions: Vec<Omission>,
    pub coverage: CaptureCoverage,
}
pub fn select_conversation(
    capture: &ConversationCapture,
    selection: &ConversationSelection,
) -> EvidenceResult<ConversationProjection> {
    selection.validate()?;
    let mut omissions = Vec::new();
    let window: BTreeSet<&RecordId> = match selection.turns {
        TurnWindow::All => capture
            .records
            .iter()
            .map(|record| &record.record_id)
            .collect(),
        TurnWindow::Last { count } => capture
            .turns
            .iter()
            .rev()
            .take(count.get())
            .flat_map(|turn| turn.records.iter())
            .collect(),
        TurnWindow::SinceCursor => capture
            .records
            .iter()
            .filter(|record| {
                capture.previous_cursor.as_ref().is_none_or(|cursor| {
                    if record.provenance == RecordProvenance::HookSupplement {
                        cursor.supplement_digest != capture.cursor.supplement_digest
                    } else {
                        cursor.record_watermarks.get(&record.record_id)
                            != capture.cursor.record_watermarks.get(&record.record_id)
                    }
                })
            })
            .map(|record| &record.record_id)
            .collect(),
    };
    let final_assistant = capture
        .records
        .iter()
        .rev()
        .find(|record| {
            window.contains(&record.record_id)
                && matches!(record.body, ConversationBody::AssistantMessage { .. })
        })
        .map(|record| &record.record_id);
    let mut records = Vec::new();
    let mut window_omitted = 0;
    let mut projection_omitted = 0;
    for record in &capture.records {
        if !window.contains(&record.record_id) {
            window_omitted += 1;
            continue;
        }
        if selection.hook_injected == Inclusion::Excluded
            && record.injection == InjectionOrigin::KnownInjected
        {
            projection_omitted += 1;
            continue;
        }
        let include = match &record.body {
            ConversationBody::UserMessage { .. } => selection.user_messages == Inclusion::Included,
            ConversationBody::AssistantMessage { .. } => match selection.assistant_messages {
                AssistantSelection::Excluded => false,
                AssistantSelection::All => true,
                AssistantSelection::FinalOnly => Some(&record.record_id) == final_assistant,
            },
            ConversationBody::ToolUse(tool) => match &selection.tool_use {
                ToolUseSelection::Excluded => false,
                ToolUseSelection::Included { tools, .. } => match tools {
                    ToolFilter::All => true,
                    ToolFilter::Named { names } => names.contains(&tool.tool_name),
                    ToolFilter::Categories { categories } => categories.contains(&tool.category),
                },
            },
        };
        if !include {
            projection_omitted += 1;
            continue;
        }
        let mut projected = record.clone();
        if let (
            ConversationBody::ToolUse(tool),
            ToolUseSelection::Included {
                results: Inclusion::Excluded,
                ..
            },
        ) = (&mut projected.body, &selection.tool_use)
        {
            tool.result = ToolResult::Absent;
        }
        records.push(projected);
    }
    if window_omitted > 0 {
        omissions.push(Omission {
            reason: OmissionReason::Window,
            count: window_omitted,
        });
    }
    if projection_omitted > 0 {
        omissions.push(Omission {
            reason: OmissionReason::Projection,
            count: projection_omitted,
        });
    }
    Ok(ConversationProjection {
        records,
        omissions,
        coverage: capture.coverage.clone(),
    })
}
