#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../../sunstack-rules/tests/support/controlled_http.rs"]
pub mod controlled;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{sync::oneshot, task::JoinHandle};

#[derive(Clone, Copy)]
pub enum RouterReply {
    ReviewHandback,
    NoRoute,
    Uncertain,
}
#[derive(Clone, Copy)]
pub enum ReplyMode {
    Answers {
        probability: f64,
        router: RouterReply,
    },
    ProviderFailure {
        status: u16,
    },
    InvalidResponse,
}
pub fn response_body(body: &Value, probability: f64, router: RouterReply) -> Value {
    let mut answers = serde_json::Map::new();
    for (name, question) in body["questions"].as_object().unwrap() {
        let answer = match question["type"].as_str().unwrap() {
            "noul" => json!({"type":"noul","noul":probability}),
            "choice" => {
                let criteria = question["criteria"].as_object().unwrap();
                let is_router = criteria.contains_key("review_handback");
                let label = if is_router {
                    match router {
                        RouterReply::ReviewHandback | RouterReply::Uncertain => "review_handback",
                        RouterReply::NoRoute => "other",
                    }
                } else if criteria.contains_key("both") {
                    "both"
                } else if criteria.contains_key("none") {
                    "none"
                } else {
                    criteria.keys().next().unwrap().as_str()
                };
                let probabilities: serde_json::Map<String, Value> = criteria
                    .keys()
                    .map(|key| (key.clone(), json!(if key == label { 1.0 } else { 0.0 })))
                    .collect();
                json!({"type":"choice","choice":label,"confidence":if is_router && matches!(router, RouterReply::Uncertain) {0.4} else {1.0},"probabilities":probabilities})
            }
            "score" => {
                let criteria = question["criteria"].as_array().unwrap();
                let last = criteria.len() - 1;
                let legend: serde_json::Map<String, Value> = criteria
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| (index.to_string(), entry.clone()))
                    .collect();
                let probabilities: serde_json::Map<String, Value> = criteria
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        (
                            index.to_string(),
                            json!(if index == last { 1.0 } else { 0.0 }),
                        )
                    })
                    .collect();
                json!({"type":"score","score":f64::from(u32::try_from(last).unwrap()),"confidence":1.0,"legend":legend,"probabilities":probabilities})
            }
            other => panic!("unexpected primitive {other}"),
        };
        answers.insert(name.clone(), answer);
    }
    json!({"model":"typesafe/controlled-journey","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}})
}
pub struct AnswerServer {
    pub endpoint: String,
    observed: Arc<AtomicUsize>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<(usize, Vec<Value>)>,
}
impl AnswerServer {
    pub async fn start(mode: ReplyMode) -> Self {
        let mut server = controlled::ControlledServer::start().await;
        let endpoint = server.endpoint.clone();
        let observed = Arc::new(AtomicUsize::new(0));
        let counter = observed.clone();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut bodies = Vec::new();
            loop {
                let request = tokio::select! {biased; _ = &mut stopped => break, request = server.next() => request};
                bodies.push(request.body.clone());
                counter.fetch_add(1, Ordering::SeqCst);
                match mode {
                    ReplyMode::Answers {
                        probability,
                        router,
                    } => {
                        let body = response_body(&request.body, probability, router);
                        request.respond(200, body);
                    }
                    ReplyMode::ProviderFailure { status } => {
                        request.respond(status, json!({"error":"PROVIDER_ERROR_SENTINEL"}))
                    }
                    ReplyMode::InvalidResponse => {
                        request.respond(200, json!("PROVIDER_BAD_JSON_SENTINEL"))
                    }
                }
            }
            (server.finish().await, bodies)
        });
        Self {
            endpoint,
            observed,
            stop,
            task,
        }
    }
    pub fn count(&self) -> usize {
        self.observed.load(Ordering::SeqCst)
    }
    pub async fn finish(self) -> (usize, Vec<Value>) {
        let _ = self.stop.send(());
        let result = self.task.await.unwrap();
        assert_eq!(result.0, result.1.len());
        result
    }
}
