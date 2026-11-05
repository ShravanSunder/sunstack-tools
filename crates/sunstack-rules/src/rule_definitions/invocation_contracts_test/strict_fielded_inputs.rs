#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate as rules;
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
    definition_selections_stay_closed,
    rules::InvocationDefinitions,
    json!({"kind":"router","path":"router.json","binding_groups":[{"bindings":{"subject":{"kind":"payload","value":{}}}}]}),
    json!({"kind":"rule_sets","rule_sets":[{"path":"set.json","binding_groups":[{"bindings":{"subject":{"kind":"document","repository":"repo","path":"source.md"}}}]}]})
);
fielded_cases!(
    input_bindings_stay_closed,
    rules::InputBinding,
    json!({"kind":"document","repository":"repo","path":"source.md"}),
    json!({"kind":"conversation","transcript_path":"session.jsonl","format":"claude_code_jsonl"}),
    json!({"kind":"payload","value":{"unexpected":1}}),
    json!({"kind":"change_set","repository":"repo"})
);
