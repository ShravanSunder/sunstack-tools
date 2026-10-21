use crate::{EntryName, LintConfig, RuleSelector, SeveritySetting};
use serde::{Deserialize, Serialize};
use std::path::Path;
use sunstack_rules::{RuleId, Severity};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettingOrigin {
    Authored,
    EntryWildcard,
    ExactRule,
    Override { index: u32 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveSetting {
    pub setting: SeveritySetting,
    pub decided_by: SettingOrigin,
}
pub fn effective_setting(
    config: &LintConfig,
    entry: &EntryName,
    rule: &RuleId,
    authored: Severity,
    file: Option<&Path>,
) -> EffectiveSetting {
    let mut result = EffectiveSetting {
        setting: match authored {
            Severity::Error => SeveritySetting::Error,
            Severity::Warn => SeveritySetting::Warn,
        },
        decided_by: SettingOrigin::Authored,
    };
    let wildcard = RuleSelector::AllInEntry {
        entry: entry.clone(),
    };
    let exact = RuleSelector::Exact {
        entry: entry.clone(),
        rule: rule.clone(),
    };
    if let Some(setting) = config.rules.get(&wildcard) {
        result = EffectiveSetting {
            setting: *setting,
            decided_by: SettingOrigin::EntryWildcard,
        };
    }
    if let Some(setting) = config.rules.get(&exact) {
        result = EffectiveSetting {
            setting: *setting,
            decided_by: SettingOrigin::ExactRule,
        };
    }
    if let Some(file) = file {
        for (index, block) in config.overrides.iter().enumerate() {
            if block.files.iter().any(|pattern| {
                globset::Glob::new(pattern.as_str())
                    .is_ok_and(|glob| glob.compile_matcher().is_match(file))
            }) && let Some(setting) = block
                .rules
                .get(&exact)
                .or_else(|| block.rules.get(&wildcard))
            {
                result = EffectiveSetting {
                    setting: *setting,
                    decided_by: SettingOrigin::Override {
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                    },
                };
            }
        }
    }
    result
}
