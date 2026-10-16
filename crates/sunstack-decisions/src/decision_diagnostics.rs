use crate::decision_contracts::ProviderFailureClass;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDiagnosticCode {
    UnknownField,
    DuplicateField,
    InvalidQuestion,
    InvalidState,
    UnsupportedPrimitive,
    InvalidNumber,
    InvalidRubric,
    InvalidConfiguration,
    ContextBudgetExceeded,
    BudgetUnverifiable,
    UnsupportedBudgetProfile,
    InvalidBody,
    MissingAnswer,
    UnexpectedAnswer,
    WrongAnswerKind,
    InvalidChoice,
    InvalidReceipt,
    ProviderRejectedRequest,
    Authentication,
    Authorization,
    InsufficientCredits,
    ProviderResourceUnavailable,
    RequestTooLarge,
    RateLimit,
    ProviderUnavailable,
    Timeout,
    Connection,
    HttpError,
    UnexpectedProviderError,
    CallerAborted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDiagnosticOrigin {
    Request,
    Response,
    Provider,
    Execution,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionDiagnosticLocation {
    None,
    Field { field_path: Vec<String> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetryAdvice {
    RetryAfter { milliseconds: u64 },
    RepairRequest,
    RepairSource,
    CaptureAgain,
    RepairConfiguration,
    RestoreProviderAccess,
    DoNotRetry,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{code:?}: {guidance}")]
#[serde(deny_unknown_fields)]
pub struct DecisionDiagnostic {
    pub origin: DecisionDiagnosticOrigin,
    pub code: DecisionDiagnosticCode,
    pub location: DecisionDiagnosticLocation,
    pub guidance: String,
    pub retry_advice: RetryAdvice,
}
impl DecisionDiagnostic {
    pub fn request(code: DecisionDiagnosticCode) -> Self {
        Self::new(
            DecisionDiagnosticOrigin::Request,
            code,
            RetryAdvice::RepairRequest,
        )
    }
    pub fn response(code: DecisionDiagnosticCode) -> Self {
        let mut diagnostic = Self::new(
            DecisionDiagnosticOrigin::Response,
            code,
            RetryAdvice::DoNotRetry,
        );
        let field = if code == DecisionDiagnosticCode::InvalidReceipt {
            "receipt"
        } else {
            "answers"
        };
        diagnostic.location = DecisionDiagnosticLocation::Field {
            field_path: vec![field.to_owned()],
        };
        diagnostic
    }
    pub fn new(
        origin: DecisionDiagnosticOrigin,
        code: DecisionDiagnosticCode,
        retry_advice: RetryAdvice,
    ) -> Self {
        let guidance = match code {
            DecisionDiagnosticCode::ContextBudgetExceeded => "Reduce the request or raise its configured finite budget.",
            DecisionDiagnosticCode::BudgetUnverifiable | DecisionDiagnosticCode::UnsupportedBudgetProfile => "Supply supported accounting with a known limit; permit estimated accounting explicitly if appropriate.",
            DecisionDiagnosticCode::CallerAborted => "The caller cancelled. Remote inference may already have happened.",
            DecisionDiagnosticCode::ProviderRejectedRequest => "Repair the request shape or selected model's supported primitives.",
            DecisionDiagnosticCode::Authentication => "Supply a valid provider credential through execution configuration.",
            DecisionDiagnosticCode::Authorization => "Restore permission to access the selected provider resource.",
            DecisionDiagnosticCode::InsufficientCredits => "Restore provider credits before sending another request.",
            DecisionDiagnosticCode::ProviderResourceUnavailable => "Check that the selected endpoint and model are available to this account.",
            DecisionDiagnosticCode::RequestTooLarge => "Reduce the encoded request size; required evidence must remain complete.",
            DecisionDiagnosticCode::RateLimit => "Wait for the advised bounded delay before retrying the request.",
            DecisionDiagnosticCode::ProviderUnavailable => "The provider is unavailable; retry only within the configured attempt budget.",
            DecisionDiagnosticCode::Timeout => "The deadline elapsed. Remote inference may have happened; retry only within the configured attempt budget.",
            DecisionDiagnosticCode::Connection => "Check transport access. Remote inference may have happened despite the connection failure.",
            DecisionDiagnosticCode::InvalidConfiguration => "Repair the backend or finite attempt settings before dispatch.",
            DecisionDiagnosticCode::InvalidReceipt => "Require a resolved model, native nonnegative token counts and a valid optional cost.",
            _ => "Repair the declared request or retry only as advised; received values and provider details are withheld.",
        }.to_owned();
        Self {
            origin,
            code,
            location: DecisionDiagnosticLocation::None,
            guidance,
            retry_advice,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticList {
    pub items: Vec<DecisionDiagnostic>,
    pub omitted_count: usize,
}
impl From<DecisionDiagnostic> for DiagnosticList {
    fn from(diagnostic: DecisionDiagnostic) -> Self {
        Self {
            items: vec![diagnostic],
            omitted_count: 0,
        }
    }
}

pub fn classify_http_status(status: u16) -> ProviderFailureClass {
    match status {
        400 => ProviderFailureClass::ProviderRejectedRequest,
        401 => ProviderFailureClass::Authentication,
        402 => ProviderFailureClass::InsufficientCredits,
        403 => ProviderFailureClass::Authorization,
        404 => ProviderFailureClass::ProviderResourceUnavailable,
        413 => ProviderFailureClass::RequestTooLarge,
        429 => ProviderFailureClass::RateLimit,
        524 => ProviderFailureClass::Timeout,
        500..=599 => ProviderFailureClass::ProviderUnavailable,
        _ => ProviderFailureClass::HttpError,
    }
}
pub fn provider_diagnostic(class: ProviderFailureClass) -> DecisionDiagnostic {
    use DecisionDiagnosticCode as Code;
    let code = match class {
        ProviderFailureClass::ProviderRejectedRequest => Code::ProviderRejectedRequest,
        ProviderFailureClass::Authentication => Code::Authentication,
        ProviderFailureClass::Authorization => Code::Authorization,
        ProviderFailureClass::InsufficientCredits => Code::InsufficientCredits,
        ProviderFailureClass::ProviderResourceUnavailable => Code::ProviderResourceUnavailable,
        ProviderFailureClass::RequestTooLarge => Code::RequestTooLarge,
        ProviderFailureClass::RateLimit => Code::RateLimit,
        ProviderFailureClass::ProviderUnavailable => Code::ProviderUnavailable,
        ProviderFailureClass::Timeout => Code::Timeout,
        ProviderFailureClass::Connection => Code::Connection,
        ProviderFailureClass::HttpError => Code::HttpError,
        ProviderFailureClass::UnexpectedProviderError => Code::UnexpectedProviderError,
    };
    let advice = match class {
        ProviderFailureClass::RateLimit
        | ProviderFailureClass::ProviderUnavailable
        | ProviderFailureClass::Timeout
        | ProviderFailureClass::Connection => RetryAdvice::RetryAfter {
            milliseconds: 1_000,
        },
        ProviderFailureClass::Authentication
        | ProviderFailureClass::Authorization
        | ProviderFailureClass::InsufficientCredits
        | ProviderFailureClass::ProviderResourceUnavailable => RetryAdvice::RestoreProviderAccess,
        ProviderFailureClass::ProviderRejectedRequest | ProviderFailureClass::RequestTooLarge => {
            RetryAdvice::RepairRequest
        }
        ProviderFailureClass::HttpError | ProviderFailureClass::UnexpectedProviderError => {
            RetryAdvice::Unknown
        }
    };
    DecisionDiagnostic::new(DecisionDiagnosticOrigin::Provider, code, advice)
}
