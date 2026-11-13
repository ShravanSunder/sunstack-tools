use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod github_heading_slug;

/// The configured renderer convention for Markdown heading anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HeadingAnchorScheme {
    Github,
}

pub(crate) enum HeadingAnchorSlugger {
    Github(github_heading_slug::GithubHeadingSlugger),
}

impl HeadingAnchorScheme {
    pub(crate) fn slugger(self) -> HeadingAnchorSlugger {
        match self {
            Self::Github => {
                HeadingAnchorSlugger::Github(github_heading_slug::GithubHeadingSlugger::new())
            }
        }
    }
}

impl HeadingAnchorSlugger {
    pub(crate) fn slug(&mut self, text: &str) -> String {
        match self {
            Self::Github(slugger) => slugger.slug(text),
        }
    }
}
