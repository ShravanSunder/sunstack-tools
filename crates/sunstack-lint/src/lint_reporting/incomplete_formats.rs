use crate::{IncompleteReason, SelectionEntry};

pub(crate) fn incomplete_text(reason: &IncompleteReason) -> String {
    match reason {
        IncompleteReason::ChoiceOptionsExceeded { options, limit } => {
            return format!("choice has {options} options; maximum is {limit}");
        }
        IncompleteReason::Filter { filter, reason } => {
            return format!(
                "filter {}: {}",
                filter.as_str(),
                serde_json::to_string(reason).unwrap_or_else(|_| "incomplete".into())
            );
        }
        IncompleteReason::NoFilterFolder { filter } => {
            return format!("filter {}: no folder", filter.as_str());
        }
        IncompleteReason::SelectionUnavailable {
            evidence_kind,
            file,
            selection_kind,
        } => {
            return format!(
                "{} [{}]: selection {} unavailable",
                file.display(),
                evidence_kind.as_str(),
                selection_kind.as_str()
            );
        }
        IncompleteReason::Unavailable { reason } => return unavailable_text(reason),
        IncompleteReason::SelectionFailed => "selection failed",
        IncompleteReason::SelectionIncomplete => "selection incomplete",
        IncompleteReason::EvaluationFailed => "evaluation failed",
        IncompleteReason::NotStarted => "not started",
        IncompleteReason::PreparationFailed => "preparation failed",
        IncompleteReason::CoveragePartial => "partial rule coverage",
        IncompleteReason::CoverageNone => "no rule coverage",
        IncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        } => {
            let class = provider_failure_name(*failure_class);
            return match http_status {
                Some(status) => format!("provider failure ({class}, HTTP {status})"),
                None => format!("provider failure ({class})"),
            };
        }
        IncompleteReason::InvalidRequest => "invalid request",
        IncompleteReason::InvalidResponse => "invalid response",
        IncompleteReason::RequestLimitReached => "request limit reached",
        IncompleteReason::EvidenceUnavailable => "evidence unavailable",
        IncompleteReason::EvidenceTooLarge => "evidence too large",
        IncompleteReason::RequestTooLarge => "request too large",
    }
    .into()
}

pub(crate) fn unavailable_text(reason: &sunstack_rules::WorkUnavailableReason) -> String {
    use sunstack_rules::WorkUnavailableReason;
    match reason {
        WorkUnavailableReason::CredentialUnavailable {} => "model credential unavailable".into(),
        WorkUnavailableReason::FilterUnavailable { filter, folder } => format!(
            "filter {} unavailable in {}",
            filter.as_str(),
            folder.display()
        ),
        WorkUnavailableReason::SelectionUnavailable {
            evidence_kind,
            file,
        } => format!(
            "selection {} unavailable in {}",
            evidence_kind.as_str(),
            file.display()
        ),
        WorkUnavailableReason::SelectedFieldUnavailable { field } => {
            format!("selected field {} unavailable", field.as_str())
        }
    }
}

pub(crate) fn selection_incomplete_text(entry: &SelectionEntry) -> Option<String> {
    use sunstack_rules::{SelectionIncompleteReason, SelectionOutcome};
    if let SelectionOutcome::Unavailable { reason } = &entry.outcome {
        return Some(format!(
            "{}:{} [{}]: selection unavailable ({})",
            entry.location.file.display(),
            entry.location.first_line,
            entry.kind.as_str(),
            unavailable_text(reason)
        ));
    }
    let SelectionOutcome::Incomplete { reason } = &entry.outcome else {
        return None;
    };
    let reason = match reason {
        SelectionIncompleteReason::EvidenceUnavailable { .. } => {
            IncompleteReason::EvidenceUnavailable
        }
        SelectionIncompleteReason::EvidenceTooLarge => IncompleteReason::EvidenceTooLarge,
        SelectionIncompleteReason::RequestTooLarge { .. } => IncompleteReason::RequestTooLarge,
        SelectionIncompleteReason::InvalidRequest => IncompleteReason::InvalidRequest,
        SelectionIncompleteReason::ProviderFailure {
            failure_class,
            http_status,
        } => IncompleteReason::ProviderFailure {
            failure_class: *failure_class,
            http_status: *http_status,
        },
        SelectionIncompleteReason::InvalidResponse => IncompleteReason::InvalidResponse,
        SelectionIncompleteReason::RequestLimitReached => IncompleteReason::RequestLimitReached,
        SelectionIncompleteReason::Cancelled => IncompleteReason::NotStarted,
    };
    Some(format!(
        "{}:{} [{}]: selection incomplete ({})",
        entry.location.file.display(),
        entry.location.first_line,
        entry.kind.as_str(),
        incomplete_text(&reason)
    ))
}

fn provider_failure_name(class: sunstack_decisions::ProviderFailureClass) -> &'static str {
    use sunstack_decisions::ProviderFailureClass;
    match class {
        ProviderFailureClass::ProviderRejectedRequest => "provider_rejected_request",
        ProviderFailureClass::Authentication => "authentication",
        ProviderFailureClass::Authorization => "authorization",
        ProviderFailureClass::InsufficientCredits => "insufficient_credits",
        ProviderFailureClass::ProviderResourceUnavailable => "provider_resource_unavailable",
        ProviderFailureClass::RequestTooLarge => "request_too_large",
        ProviderFailureClass::RateLimit => "rate_limit",
        ProviderFailureClass::ProviderUnavailable => "provider_unavailable",
        ProviderFailureClass::Timeout => "timeout",
        ProviderFailureClass::Connection => "connection",
        ProviderFailureClass::HttpError => "http_error",
        ProviderFailureClass::UnexpectedProviderError => "unexpected_provider_error",
    }
}
