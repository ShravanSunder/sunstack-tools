use std::fmt;

/// A configuration location whose authored names are never parsed as separators.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigPath(Vec<ConfigPathSegment>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigPathSegment {
    Key(String),
    Index(usize),
}

impl ConfigPath {
    pub fn new(segments: impl IntoIterator<Item = ConfigPathSegment>) -> Self {
        Self(segments.into_iter().collect())
    }

    pub fn keys(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::new(
            keys.into_iter()
                .map(|key| ConfigPathSegment::Key(key.into())),
        )
    }

    pub fn with_key(&self, key: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(ConfigPathSegment::Key(key.into()));
        path
    }

    pub fn with_index(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(ConfigPathSegment::Index(index));
        path
    }

    pub fn segments(&self) -> &[ConfigPathSegment] {
        &self.0
    }
}

impl From<&ConfigPath> for ConfigPath {
    fn from(path: &ConfigPath) -> Self {
        path.clone()
    }
}

impl fmt::Display for ConfigPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, segment) in self.0.iter().enumerate() {
            if position != 0 {
                formatter.write_str(".")?;
            }
            match segment {
                ConfigPathSegment::Key(key) => formatter.write_str(key)?,
                ConfigPathSegment::Index(index) => write!(formatter, "{index}")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_preserves_public_spelling_without_collapsing_structured_identity() {
        let literal = ConfigPath::keys(["evidence", "kinds", "a.fields.body"]);
        let nested = ConfigPath::keys(["evidence", "kinds", "a", "fields", "body"]);
        assert_eq!(literal.to_string(), "evidence.kinds.a.fields.body");
        assert_eq!(literal.to_string(), nested.to_string());
        assert_ne!(literal, nested);
        let indexed = literal.with_key("within").with_index(1);
        assert_eq!(indexed.to_string(), "evidence.kinds.a.fields.body.within.1");
        assert_eq!(
            indexed.segments().last(),
            Some(&ConfigPathSegment::Index(1))
        );
    }
}
