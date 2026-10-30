use crate::{EvidenceKindDefinition, EvidenceKindName, FieldSource, MatchSource};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct EvidenceKindPlan {
    pub order: Vec<EvidenceKindName>,
    pub depends_on: BTreeMap<EvidenceKindName, BTreeSet<EvidenceKindName>>,
}
#[derive(Debug, Clone, thiserror::Error)]
#[error("{field}: {message}")]
pub struct KindPlanRefusal {
    pub field: String,
    pub message: String,
}
impl EvidenceKindPlan {
    pub fn derive(
        kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    ) -> Result<Self, Vec<KindPlanRefusal>> {
        let mut depends_on = BTreeMap::new();
        let mut errors = Vec::new();
        for (name, definition) in kinds {
            let mut dependencies = BTreeSet::new();
            if let Some(parent) = &definition.within {
                dependencies.insert(parent.clone());
            }
            for source in definition.fields.values() {
                match source {
                    FieldSource::Inside { of }
                    | FieldSource::Enclosing { of }
                    | FieldSource::SameFile { of }
                    | FieldSource::Remainder { exclude: Some(of) } => {
                        if let MatchSource::Kind { kind } = of {
                            dependencies.insert(kind.clone());
                        }
                    }
                    FieldSource::Target {
                        as_kind: Some(kind),
                        ..
                    } => {
                        dependencies.insert(kind.clone());
                    }
                    _ => {}
                }
            }
            for dependency in &dependencies {
                if !kinds.contains_key(dependency) {
                    errors.push(KindPlanRefusal {
                        field: format!("evidence.kinds.{}", name.as_str()),
                        message: format!("undefined kind {}", dependency.as_str()),
                    });
                }
            }
            depends_on.insert(name.clone(), dependencies);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut order = Vec::new();
        let mut completed = BTreeSet::new();
        while order.len() < kinds.len() {
            let next = depends_on
                .iter()
                .find(|(kind, dependencies)| {
                    !completed.contains(*kind) && dependencies.is_subset(&completed)
                })
                .map(|(kind, _)| kind.clone());
            let Some(next) = next else {
                return Err(vec![KindPlanRefusal {
                    field: "evidence.kinds".into(),
                    message: "kind dependency cycle".into(),
                }]);
            };
            completed.insert(next.clone());
            order.push(next);
        }
        for (name, definition) in kinds {
            for source in definition.fields.values() {
                if let FieldSource::Target {
                    as_kind: Some(target),
                    ..
                } = source
                    && reaches_selected(target, kinds, &depends_on, &mut BTreeSet::new())
                {
                    errors.push(KindPlanRefusal {
                        field: format!("evidence.kinds.{}", name.as_str()),
                        message: "as_kind names a selected or selection-dependent kind".into(),
                    });
                }
            }
        }
        if errors.is_empty() {
            Ok(Self { order, depends_on })
        } else {
            Err(errors)
        }
    }
}
fn reaches_selected(
    kind: &EvidenceKindName,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    dependencies: &BTreeMap<EvidenceKindName, BTreeSet<EvidenceKindName>>,
    visited: &mut BTreeSet<EvidenceKindName>,
) -> bool {
    if !visited.insert(kind.clone()) {
        return false;
    }
    kinds
        .get(kind)
        .is_some_and(|definition| definition.select.is_some())
        || dependencies.get(kind).is_some_and(|parents| {
            parents
                .iter()
                .any(|parent| reaches_selected(parent, kinds, dependencies, visited))
        })
}
#[cfg(test)]
mod tests;
