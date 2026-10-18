use crate::{
    DiagnosticList, SkippedRule, definition_admission::locate_at,
    invocation_admission::AdmittedGroup,
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn admit_markdown_entries(groups: &mut [AdmittedGroup]) {
    for group in groups {
        let sources = group
            .sources
            .iter()
            .map(|(name, source)| (name.clone(), source.spec.clone()))
            .collect::<BTreeMap<_, _>>();
        let set = Arc::make_mut(&mut group.set);
        for located in std::mem::take(&mut set.rules) {
            let Some(provenance) = &located.provenance else {
                set.rules.push(located);
                continue;
            };
            let mut diagnostics = DiagnosticList::default();
            for declaration in &located.definition.evidence {
                if let Err(error) =
                    sunstack_evidence::admit_evidence_declaration(declaration, &sources)
                {
                    diagnostics.extend(error.diagnostics().into());
                }
            }
            if diagnostics.is_empty() {
                set.rules.push(located);
            } else {
                locate_at(
                    &mut diagnostics,
                    &provenance.rules,
                    provenance.entry_location.line,
                    provenance.entry_location.column,
                );
                set.skipped.push(SkippedRule::Entry {
                    rule_set: set.path.clone(),
                    markdown: provenance.markdown.clone(),
                    rules: provenance.rules.clone(),
                    key: provenance.marker_id.as_str().into(),
                    rule_id: Some(located.definition.rule_id.clone()),
                    diagnostics,
                });
            }
        }
    }
}
