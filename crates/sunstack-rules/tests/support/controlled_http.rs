#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::time::Duration;
use sunstack_decisions::CancellationToken;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    task::{JoinHandle, JoinSet},
};

pub struct PendingHttpRequest {
    pub body: Value,
    reply: oneshot::Sender<(u16, Value)>,
}
impl PendingHttpRequest {
    pub fn respond(self, status: u16, body: Value) {
        let _ = self.reply.send((status, body));
    }
    pub fn noul(self, probability: f64) {
        let answers = self.body["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(|name| (name.clone(), json!({"type":"noul","noul":probability})))
            .collect::<serde_json::Map<_, _>>();
        self.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}));
    }
}
pub struct ControlledServer {
    pub endpoint: String,
    requests: mpsc::UnboundedReceiver<PendingHttpRequest>,
    cancellation: CancellationToken,
    task: JoinHandle<usize>,
}
impl ControlledServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/systemone", listener.local_addr().unwrap());
        let (send, requests) = mpsc::unbounded_channel();
        let cancellation = CancellationToken::new();
        let stopped = cancellation.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {biased;_=stopped.cancelled()=>break,
                    accepted=listener.accept()=>{
                        let (connection,_)=accepted.unwrap();let send=send.clone();let stopped=stopped.clone();
                        connections.spawn(async move {tokio::select! {biased;_=stopped.cancelled()=>0, count=serve(connection,send)=>count}});
                    }
                }
            }
            let mut count = 0;
            while let Some(result) = connections.join_next().await {
                count += result.unwrap();
            }
            count
        });
        Self {
            endpoint,
            requests,
            cancellation,
            task,
        }
    }
    pub async fn next(&mut self) -> PendingHttpRequest {
        tokio::time::timeout(Duration::from_secs(3), self.requests.recv())
            .await
            .expect("HTTP boundary was not reached")
            .expect("server stopped")
    }
    pub async fn finish(self) -> usize {
        self.cancellation.cancel();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
    }
}
async fn serve(mut stream: TcpStream, send: mpsc::UnboundedSender<PendingHttpRequest>) -> usize {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 2048];
    let (header_end, length) = loop {
        let count = stream.read(&mut buffer).await.unwrap_or(0);
        if count == 0 {
            return 0;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let length = String::from_utf8_lossy(&bytes[..end])
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            break (end + 4, length);
        }
        assert!(bytes.len() < 1_000_000);
    };
    while bytes.len() < header_end + length {
        let count = stream.read(&mut buffer).await.unwrap_or(0);
        if count == 0 {
            return 0;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    let body = serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
    let (reply, receive) = oneshot::channel();
    if send.send(PendingHttpRequest { body, reply }).is_err() {
        return 1;
    }
    if let Ok((status, body)) = receive.await {
        let bytes = serde_json::to_vec(&body).unwrap();
        let header = format!(
            "HTTP/1.1 {status} Controlled\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        if stream.write_all(header.as_bytes()).await.is_ok() {
            let _ = stream.write_all(&bytes).await;
        }
    }
    1
}
