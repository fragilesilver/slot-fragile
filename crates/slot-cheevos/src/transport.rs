// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/transport.rs.

use std::io::Read;
use std::time::Duration;

use crate::ffi::RETRYABLE_CLIENT_ERROR;

const MAX_BODY: u64 = 8 << 20;
const MAX_IMAGE: u64 = 1 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub post: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: i32,
    pub body: Vec<u8>,
}

impl Response {
    pub fn unreachable() -> Self {
        Self {
            status: RETRYABLE_CLIENT_ERROR,
            body: Vec::new(),
        }
    }
}

pub trait Transport: Send + Sync {
    fn send(&self, request: &Request) -> Response;
    fn fetch(&self, url: &str) -> Option<Vec<u8>>;
}

pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    pub fn new(user_agent: &str) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(10))
                .timeout_read(Duration::from_secs(30))
                .timeout_write(Duration::from_secs(30))
                .user_agent(user_agent)
                .build(),
        }
    }
}

fn body(response: ureq::Response, cap: u64) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    response
        .into_reader()
        .take(cap)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

impl Transport for Http {
    fn send(&self, request: &Request) -> Response {
        let sent = match &request.post {
            Some(post) => self
                .agent
                .post(&request.url)
                .set(
                    "Content-Type",
                    request
                        .content_type
                        .as_deref()
                        .unwrap_or("application/x-www-form-urlencoded"),
                )
                .send_string(post),
            None => self.agent.get(&request.url).call(),
        };
        let reply = match sent {
            Ok(r) => r,
            Err(ureq::Error::Status(_, r)) => r,
            Err(e) => {
                eprintln!("slot: cheevos: {}", e.kind());
                return Response::unreachable();
            }
        };
        let status = reply.status() as i32;
        match body(reply, MAX_BODY) {
            Some(body) => Response { status, body },
            None => Response::unreachable(),
        }
    }

    fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        let reply = self
            .agent
            .get(url)
            .timeout(Duration::from_secs(5))
            .call()
            .ok()?;
        body(reply, MAX_IMAGE)
    }
}
