use super::evidence_kind_contracts::{EvidenceKindDefinition, FieldName, FieldShape, FieldSource};
use std::collections::BTreeSet;

impl FieldSource {
    pub fn containing_field(&self) -> Option<&FieldName> {
        match self {
            Self::SameFile { containing, .. } | Self::Document { containing, .. } => {
                containing.as_ref()
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "field_filter_dependencies_test.rs"]
mod tests;

impl EvidenceKindDefinition {
    pub(crate) fn containing_field_order(&self) -> Result<Vec<&FieldName>, String> {
        let mut order = Vec::new();
        let mut remaining = BTreeSet::new();
        for (name, source) in &self.fields {
            if let Some(needle) = source.containing_field() {
                if !self.fields.get(needle).is_some_and(|source| {
                    matches!(source.shape(), FieldShape::Text | FieldShape::Texts)
                }) {
                    return Err(format!(
                        "field {} containing must name a defined text or text-list field",
                        name.as_str()
                    ));
                }
                remaining.insert(name);
            }
        }
        while !remaining.is_empty() {
            let next = remaining.iter().copied().find(|name| {
                self.fields[*name]
                    .containing_field()
                    .is_none_or(|needle| !remaining.contains(needle))
            });
            let Some(next) = next else {
                return Err("containing field dependency cycle".into());
            };
            remaining.remove(next);
            order.push(next);
        }
        Ok(order)
    }

    pub fn field_needs_document(&self, name: &FieldName) -> bool {
        let mut current = Some(name);
        let mut visited = BTreeSet::new();
        while let Some(name) = current {
            if !visited.insert(name) {
                break;
            }
            let Some(source) = self.fields.get(name) else {
                break;
            };
            if matches!(source, FieldSource::Document { .. }) {
                return true;
            }
            current = source.containing_field();
        }
        false
    }
}
