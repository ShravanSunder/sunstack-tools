#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate as evidence;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
fn assert_closed_fielded<TInput: DeserializeOwned + Serialize>(valid: Value) {
    let decoded: TInput = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);
    let mut extra = valid;
    extra["unexpected"] = json!(1);
    assert!(
        serde_json::from_value::<TInput>(extra).is_err(),
        "{} accepted unexpected",
        std::any::type_name::<TInput>()
    );
}
macro_rules! fielded_cases {
    ($name:ident, $input:ty, $($case:expr),+ $(,)?) => {
        #[test]
        fn $name() { $(assert_closed_fielded::<$input>($case);)+ }
    };
}
fielded_cases!(
    native_selectors_stay_closed,
    evidence::EvidenceSelector,
    json!({"kind":"paragraph","repository_id":"repo","source_path":"source.md","target_id":"U001"}),
    json!({"kind":"file_text","repository_id":"repo","source_path":"source.md"}),
    json!({"kind":"literal_text","repository_id":"repo","paths":[{"kind":"exact","path":"source.md"}],"text":"text"}),
    json!({"kind":"regex_text","repository_id":"repo","paths":[{"kind":"glob","pattern":"*.md"}],"pattern":"text"}),
    json!({"kind":"ast_pattern","repository_id":"repo","paths":[{"kind":"glob","pattern":"*.rs"}],"language":"rust","pattern":"fn $NAME() {}"})
);
