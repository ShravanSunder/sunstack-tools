use crate::definition_contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf};
use sunstack_decisions::{AttemptSettings, BackendSelection, RequestBudgetSettings};
use sunstack_evidence::{
    InvocationLimits, PayloadSchemaPath, RepositoryBinding, RepositoryId, ReturnedCursor,
    SourceName, StructuredJson, TranscriptFormatName,
};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleInvocationInput {
    pub definition_root: PathBuf,
    pub definitions: InvocationDefinitions,
    pub repositories: Vec<RepositoryBinding>,
    pub payload_schemas: Vec<PayloadSchemaPath>,
    pub origin: InvocationOrigin,
    pub cursors: Vec<ReturnedCursor>,
    pub execution: ExecutionSettings,
    pub limits: InvocationLimits,
    pub backend: BackendSelection,
    pub attempts: AttemptSettings,
    pub request_budget: RequestBudgetSettings,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvocationDefinitions {
    Router {
        path: PathBuf,
        #[schemars(length(min = 1))]
        binding_groups: Vec<BindingGroup>,
    },
    RuleSets {
        #[schemars(length(min = 1))]
        rule_sets: Vec<BoundRuleSet>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundRuleSet {
    pub path: PathBuf,
    #[schemars(length(min = 1))]
    pub binding_groups: Vec<BindingGroup>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingGroup {
    #[schemars(extend("minProperties"=1))]
    pub bindings: BTreeMap<SourceName, InputBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputBinding {
    Document {
        repository: RepositoryId,
        path: PathBuf,
    },
    Conversation {
        transcript_path: PathBuf,
        format: TranscriptFormatName,
        #[serde(skip_serializing_if = "Option::is_none")]
        final_assistant_text: Option<String>,
    },
    Payload {
        value: StructuredJson,
    },
    ChangeSet {
        repository: RepositoryId,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Speculation {
    Off,
    On,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSettings {
    pub speculation: Speculation,
    pub max_concurrent_requests: NonZeroUsize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvocationOrigin {
    Direct {},
    Hook {
        host: String,
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        session: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        turn: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent: Option<OriginAgent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<chrono::DateTime<chrono::Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OriginAgent {
    Main {},
    Subagent {
        id: String,
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    Unknown {},
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedField<TValue> {
    Observed { value: TValue },
    Unavailable,
}
impl<TValue> From<Option<TValue>> for ObservedField<TValue> {
    fn from(value: Option<TValue>) -> Self {
        match value {
            Some(value) => Self::Observed { value },
            None => Self::Unavailable,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReportedAgent {
    Main,
    Subagent {
        id: String,
        #[serde(rename = "type")]
        agent_type: ObservedField<String>,
    },
    Unknown,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReportedOrigin {
    Direct,
    Hook {
        host: String,
        event: String,
        session: ObservedField<String>,
        turn: ObservedField<String>,
        agent: ReportedAgent,
        observed_at: ObservedField<chrono::DateTime<chrono::Utc>>,
    },
}
impl From<&InvocationOrigin> for ReportedOrigin {
    fn from(origin: &InvocationOrigin) -> Self {
        match origin {
            InvocationOrigin::Direct {} => Self::Direct,
            InvocationOrigin::Hook {
                host,
                event,
                session,
                turn,
                agent,
                observed_at,
            } => Self::Hook {
                host: host.clone(),
                event: event.clone(),
                session: session.clone().into(),
                turn: turn.clone().into(),
                observed_at: (*observed_at).into(),
                agent: match agent {
                    Some(OriginAgent::Main {}) => ReportedAgent::Main,
                    Some(OriginAgent::Subagent { id, agent_type }) => ReportedAgent::Subagent {
                        id: id.clone(),
                        agent_type: agent_type.clone().into(),
                    },
                    Some(OriginAgent::Unknown {}) => ReportedAgent::Unknown,
                    None => ReportedAgent::Unavailable,
                },
            },
        }
    }
}
impl InvocationOrigin {
    pub fn field(&self, field: OriginField) -> Option<&str> {
        match self {
            Self::Direct {} => None,
            Self::Hook {
                host, event, agent, ..
            } => match field {
                OriginField::Host => Some(host),
                OriginField::Event => Some(event),
                OriginField::AgentKind => match agent {
                    Some(OriginAgent::Main {}) => Some("main"),
                    Some(OriginAgent::Subagent { .. }) => Some("subagent"),
                    Some(OriginAgent::Unknown {}) => Some("unknown"),
                    None => None,
                },
                OriginField::AgentType => match agent {
                    Some(OriginAgent::Subagent { agent_type, .. }) => agent_type.as_deref(),
                    _ => None,
                },
            },
        }
    }
}
