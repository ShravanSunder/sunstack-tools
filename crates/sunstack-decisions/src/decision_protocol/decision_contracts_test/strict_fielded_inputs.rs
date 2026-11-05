#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate as decisions;
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
    native_questions_stay_closed,
    decisions::JudgmentQuestion,
    json!({"type":"noul","instructions":{"kind":"full","unexpected":1}}),
    json!({"type":"choice","instructions":"choose","criteria":{"a":{"unexpected":1},"b":null}}),
    json!({"type":"score","instructions":"score","criteria":["low","high"]})
);
fielded_cases!(
    backend_selection_stays_closed,
    decisions::BackendSelection,
    json!({"kind":"jev_openrouter","endpoint":"https://openrouter.ai/api/v1/systemone","model":"jev-latest"})
);
