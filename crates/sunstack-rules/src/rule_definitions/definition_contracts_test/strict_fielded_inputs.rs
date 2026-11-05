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
    verdict_declarations_stay_closed,
    rules::VerdictDeclaration,
    json!({"kind":"noul","question":{"type":"noul","instructions":"judge"},"threshold":0.8}),
    json!({"kind":"composite","dimensions":[{"question":"quality","weight":1.0}],"threshold":0.8})
);
fielded_cases!(
    applicability_entries_stay_closed,
    rules::ApplicabilityEntry,
    json!({"kind":"condition","test":{"kind":"origin_field_equals","field":"host","value":"codex"}}),
    json!({"kind":"decision","question":{"type":"noul","instructions":"gate"},"evidence":[{"id":"context","input":"subject","requirement":"required","select":{"kind":"whole_document"}}],"threshold":0.6})
);
fielded_cases!(
    condition_tests_stay_closed,
    rules::ConditionTest,
    json!({"kind":"payload_field_equals","input":"flags","pointer":"/enabled","value":true}),
    json!({"kind":"payload_field_in","input":"flags","pointer":"/enabled","values":[true,null]}),
    json!({"kind":"payload_field_present","input":"flags","pointer":"/enabled"}),
    json!({"kind":"origin_field_equals","field":"host","value":"codex"}),
    json!({"kind":"pattern_present","input":"subject","pattern":{"kind":"literal","text":"text"},"max_scan_bytes":4096})
);
fielded_cases!(
    presence_patterns_stay_closed,
    rules::PresencePattern,
    json!({"kind":"literal","text":"text"}),
    json!({"kind":"regex","pattern":"text"}),
    json!({"kind":"ast","language":"rust","pattern":"fn $NAME() {}"})
);
