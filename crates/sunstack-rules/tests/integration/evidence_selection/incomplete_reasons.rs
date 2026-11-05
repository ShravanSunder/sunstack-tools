use super::*;

#[tokio::test]
async fn selection_preserves_provider_class_status_and_invalid_response_reasons() {
    for case in [
        "rate_limit",
        "unavailable",
        "schema",
        "missing",
        "timeout",
        "connection",
    ] {
        let input = input("practices-research");
        let mut server = controlled::ControlledServer::start().await;
        let endpoint = if case == "connection" {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}/v1/systemone", listener.local_addr().unwrap())
        } else {
            server.endpoint.clone()
        };
        let mut settings = settings(&endpoint);
        settings.attempts.attempt_timeout_ms = 200;
        let mut dispatch = dispatcher(settings);
        let mut operation =
            tokio::spawn(async move { select_candidates(&input, &mut dispatch).await });
        let held = if case == "connection" {
            None
        } else {
            let request = tokio::select! {
                request = server.next() => request,
                outcome = &mut operation => panic!("{case}: selection ended before expected HTTP: {outcome:?}"),
            };
            match case {
                "rate_limit" => {
                    request.respond(429, json!({"error":"controlled rate limit"}));
                    None
                }
                "unavailable" => {
                    request.respond(503, json!({"error":"controlled unavailable"}));
                    None
                }
                "schema" => {
                    request.respond(200, json!({"answers":{"c1":{"type":"noul","noul":7.0}}}));
                    None
                }
                "missing" => {
                    request.respond(200,json!({"model":"typesafe/controlled","answers":{},"usage":{"input_tokens":1,"output_tokens":1,"cost":0.001}}));
                    None
                }
                "timeout" => Some(request),
                other => panic!("{other}"),
            }
        };
        let outcomes = tokio::time::timeout(Duration::from_secs(3), operation)
            .await
            .unwrap()
            .unwrap();
        drop(held);
        assert_eq!(server.finish().await, usize::from(case != "connection"));
        assert_eq!(outcomes.len(), 1);
        let outcome = outcomes.values().next().unwrap();
        let value = serde_json::to_value(outcome).unwrap();
        let expected = match case {
            "rate_limit" => {
                json!({"kind":"provider_failure","failure_class":"rate_limit","http_status":429})
            }
            "unavailable" => {
                json!({"kind":"provider_failure","failure_class":"provider_unavailable","http_status":503})
            }
            "timeout" => json!({"kind":"provider_failure","failure_class":"timeout"}),
            "connection" => json!({"kind":"provider_failure","failure_class":"connection"}),
            _ => json!({"kind":"invalid_response"}),
        };
        assert_eq!(
            value,
            json!({"kind":"incomplete","reason":expected}),
            "{case}"
        );
    }
}

#[tokio::test]
async fn invalid_selection_request_never_dispatches_http() {
    let mut input = input("practices-research");
    input.selection.question = serde_json::from_value(
        json!({"type":"choice","instructions":"Evaluate the candidate.","criteria":{}}),
    )
    .unwrap();
    let server = controlled::ControlledServer::start().await;
    let mut dispatch = dispatcher(settings(&server.endpoint));
    let outcomes = tokio::time::timeout(
        Duration::from_secs(3),
        select_candidates(&input, &mut dispatch),
    )
    .await
    .unwrap();
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes.values().next().unwrap(),
        SelectionOutcome::Incomplete {
            reason: SelectionIncompleteReason::InvalidRequest
        }
    ));
    assert_eq!(server.finish().await, 0);
    assert!(dispatch.records.is_empty());
}
