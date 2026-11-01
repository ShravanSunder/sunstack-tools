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
    source_specs_stay_closed,
    evidence::SourceSpec,
    json!({"kind":"document","repository":"repo","path":"source.md"}),
    json!({"kind":"conversation","transcript_path":"session.jsonl","format":"claude_code_jsonl"}),
    json!({"kind":"payload","schema":"flags","value":{"unexpected":1}}),
    json!({"kind":"change_set","repository":"repo"})
);
fielded_cases!(
    query_paths_stay_closed,
    evidence::QueryPath,
    json!({"kind":"exact","path":"source.md"}),
    json!({"kind":"glob","pattern":"*.md"})
);
