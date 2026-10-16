#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::time::Duration;
use sunstack_decisions::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub fn selection(endpoint: &str) -> BackendSelection {
    BackendSelection::JevOpenrouter {
        endpoint: endpoint.into(),
        model: ModelName::new("jev-latest").unwrap(),
    }
}
pub fn mixed_input(endpoint: &str) -> RunJudgmentsInput {
    serde_json::from_value(json!({"request":{"state":{"content":"unique-state-☃-\"escaped\"\n","nested":[true,2,null]},"questions":{
        "native_yes":{"type":"noul","instructions":{"owner":"native_yes","nested":[null,true,1],"threshold":{"description":"legitimate author content, not model policy"}},"criteria":{"true":null,"false":{"description":["no",null]}}},
        "native_choice":{"type":"choice","instructions":["classify",null],"criteria":{"alpha":{"nested":[false,3]},"beta":null}},
        "native_score":{"type":"score","instructions":"score","criteria":[null,{"description":"high"}]}
    }},"backend":selection(endpoint),"attempts":AttemptSettings::default(),"request_budget":budget()})).unwrap()
}
pub fn budget() -> RequestBudgetSettings {
    RequestBudgetSettings {
        max_input_tokens: 100_000,
        max_request_bytes: 100_000,
        max_questions: 10,
        max_requests: 10,
        safety_margin_tokens: 32,
        accounting_policy: AccountingPolicy::AllowEstimated,
    }
}
pub fn backend(endpoint: &str) -> JevOpenRouterBackend {
    JevOpenRouterBackend::new(
        selection(endpoint),
        "SENTINEL_CREDENTIAL_987",
        JevAccountingProfile::EstimatedBytes {
            context_ceiling_tokens: 262_144,
        },
    )
    .unwrap()
}
pub fn response() -> Value {
    json!({"model":"typesafe/observed-model","answers":{
        "native_yes":{"type":"noul","noul":0.75},
        "native_choice":{"type":"choice","choice":"alpha","confidence":0.4,"probabilities":{"alpha":0.6,"beta":0.3}},
        "native_score":{"type":"score","score":0.35,"confidence":0.7,"legend":{"0":null,"1":{"arbitrary":"different legend text"}},"probabilities":{"0":0.65,"1":0.35}}
    },"usage":{"input_tokens":123,"output_tokens":9,"cost":0.002,"provider_extra":"SENTINEL_METADATA"},"id":"SENTINEL_METADATA","provider":"SENTINEL_METADATA"})
}
pub struct HttpReply {
    pub status: u16,
    pub body: Vec<u8>,
    pub chunked: bool,
}
impl HttpReply {
    pub fn json(value: Value) -> Self {
        Self {
            status: 200,
            body: serde_json::to_vec(&value).unwrap(),
            chunked: false,
        }
    }
}
pub struct HttpCapture {
    pub headers: String,
    pub body: Vec<u8>,
}
pub struct TestServer {
    pub endpoint: String,
    pub captures: mpsc::UnboundedReceiver<HttpCapture>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<usize>,
}
impl TestServer {
    pub async fn start(replies: Vec<HttpReply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
        let (capture_send, captures) = mpsc::unbounded_channel();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut count = 0;
            let mut replies = replies.into_iter();
            loop {
                let connection = tokio::select! {biased; _=&mut stopped=>break, result=listener.accept()=>result.unwrap().0};
                let mut connection = connection;
                let capture =
                    tokio::time::timeout(Duration::from_secs(2), read_request(&mut connection))
                        .await
                        .unwrap();
                count += 1;
                capture_send.send(capture).unwrap();
                let reply = replies.next().expect("unexpected HTTP request");
                let header = if reply.chunked {
                    format!(
                        "HTTP/1.1 {} Test\r\nTransfer-Encoding: chunked\r\nConnection: close\r\nX-Error: SENTINEL_HEADER\r\n\r\n",
                        reply.status
                    )
                } else {
                    format!(
                        "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\nX-Error: SENTINEL_HEADER\r\n\r\n",
                        reply.status,
                        reply.body.len()
                    )
                };
                connection.write_all(header.as_bytes()).await.unwrap();
                if reply.chunked {
                    for chunk in reply.body.chunks(7) {
                        let prefix = format!("{:x}\r\n", chunk.len());
                        connection.write_all(prefix.as_bytes()).await.unwrap();
                        if connection.write_all(chunk).await.is_err() {
                            break;
                        }
                        if connection.write_all(b"\r\n").await.is_err() {
                            break;
                        }
                    }
                    let _ = connection.write_all(b"0\r\n\r\n").await;
                } else {
                    let _ = connection.write_all(&reply.body).await;
                }
            }
            count
        });
        Self {
            endpoint,
            captures,
            stop,
            task,
        }
    }
    pub async fn finish(self) -> usize {
        let _ = self.stop.send(());
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
    }
}
pub async fn read_request(connection: &mut TcpStream) -> HttpCapture {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    let (header_end, length) = loop {
        let count = connection.read(&mut buffer).await.unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap();
            break (end + 4, length);
        }
        assert!(bytes.len() < 100_000);
    };
    while bytes.len() < header_end + length {
        let count = connection.read(&mut buffer).await.unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    HttpCapture {
        headers: String::from_utf8(bytes[..header_end].to_vec()).unwrap(),
        body: bytes[header_end..header_end + length].to_vec(),
    }
}
