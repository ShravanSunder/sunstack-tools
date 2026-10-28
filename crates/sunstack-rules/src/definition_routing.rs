use crate::*;
use std::collections::BTreeMap;
use sunstack_evidence::SourceName;

pub(crate) fn validate_route_mapping(
    target: &RouteTarget,
    inputs: &BTreeMap<SourceName, InputDeclaration>,
    router: &BTreeMap<SourceName, InputDeclaration>,
) -> Result<(), DiagnosticList> {
    if target.inputs.len() != inputs.len() {
        return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring));
    }
    for (name, input) in inputs {
        let source = target.inputs.get(name).and_then(|name| router.get(name));
        let compatible = match (input, source) {
            (InputDeclaration::Conversation {}, Some(InputDeclaration::Conversation {}))
            | (InputDeclaration::Document {}, Some(InputDeclaration::Document {}))
            | (InputDeclaration::ChangeSet {}, Some(InputDeclaration::ChangeSet {})) => true,
            (
                InputDeclaration::Payload { schema: a },
                Some(InputDeclaration::Payload { schema: b }),
            ) => a == b,
            _ => false,
        };
        if !compatible {
            return Err(DiagnosticList::rule(RuleDiagnosticCode::InvalidWiring));
        }
    }
    Ok(())
}
