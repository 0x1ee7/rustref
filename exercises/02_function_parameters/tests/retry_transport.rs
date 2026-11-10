//! Part 4: dependency injection through a trait, a closure predicate, and
//! generic vs `dyn` transport parameters.

use std::collections::VecDeque;
use std::num::NonZeroU32;

use ex02_function_parameters::{
    Request, RequestBuilder, Response, Transport, TransportError, send_with_retry,
};

/// A fake transport that replays scripted results and records every URL sent.
struct Scripted {
    script: VecDeque<Result<Response, TransportError>>,
    sent: Vec<String>,
}

impl Scripted {
    fn new(script: Vec<Result<Response, TransportError>>) -> Self {
        Self {
            script: script.into(),
            sent: Vec::new(),
        }
    }
}

impl Transport for Scripted {
    fn send(&mut self, request: &Request) -> Result<Response, TransportError> {
        self.sent.push(request.url().to_string());
        self.script
            .pop_front()
            .expect("transport was called more times than the script allows")
    }
}

fn ok(status: u16) -> Result<Response, TransportError> {
    Ok(Response {
        status,
        body: Vec::new(),
    })
}

fn request() -> Request {
    RequestBuilder::new("GET", "https://x.test/r")
        .build()
        .expect("valid request")
}

fn attempts(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("non-zero attempt count")
}

#[test]
fn part4_success_on_first_attempt_returns_body_and_sends_once() {
    let mut transport = Scripted::new(vec![Ok(Response {
        status: 200,
        body: b"hi".to_vec(),
    })]);
    let got = send_with_retry(&mut transport, &request(), attempts(3), |_| true)
        .expect("first attempt succeeds");
    assert_eq!(got.status, 200);
    assert_eq!(got.body, b"hi");
    assert_eq!(
        transport.sent,
        vec!["https://x.test/r".to_string()],
        "the request reaches the transport unchanged"
    );
}

#[test]
fn part4_retries_transient_errors_until_success() {
    let mut transport = Scripted::new(vec![
        Err(TransportError::Timeout),
        Err(TransportError::Reset),
        ok(201),
    ]);
    let got = send_with_retry(&mut transport, &request(), attempts(3), |_| true)
        .expect("third attempt succeeds");
    assert_eq!(got.status, 201);
    assert_eq!(transport.sent.len(), 3, "every attempt is sent");
}

#[test]
fn part4_returns_last_error_when_attempts_are_exhausted() {
    let mut transport = Scripted::new(vec![Err(TransportError::Timeout), Err(TransportError::Reset)]);
    let got = send_with_retry(&mut transport, &request(), attempts(2), |_| true);
    assert_eq!(
        got,
        Err(TransportError::Reset),
        "the last error is returned unchanged"
    );
    assert_eq!(transport.sent.len(), 2, "no attempt beyond the limit");
}

#[test]
fn part4_predicate_returning_false_stops_immediately() {
    let mut transport = Scripted::new(vec![
        Err(TransportError::Protocol("bad status line".to_string())),
        ok(200),
    ]);
    let got = send_with_retry(&mut transport, &request(), attempts(3), |e| {
        !matches!(e, TransportError::Protocol(_))
    });
    assert_eq!(
        got,
        Err(TransportError::Protocol("bad status line".to_string()))
    );
    assert_eq!(transport.sent.len(), 1, "no retry after a refused error");
}

#[test]
fn part4_predicate_is_not_consulted_after_the_final_attempt() {
    let mut transport = Scripted::new(vec![Err(TransportError::Timeout), Err(TransportError::Reset)]);
    let mut calls = 0;
    let got = send_with_retry(&mut transport, &request(), attempts(2), |_| {
        calls += 1;
        true
    });
    assert_eq!(got, Err(TransportError::Reset));
    assert_eq!(
        calls, 1,
        "the predicate runs only when another attempt could follow"
    );
}

#[test]
fn part4_predicate_sees_each_failure_in_order() {
    let mut transport = Scripted::new(vec![
        Err(TransportError::Timeout),
        Err(TransportError::Reset),
        ok(200),
    ]);
    let mut seen: Vec<TransportError> = Vec::new();
    let got = send_with_retry(&mut transport, &request(), attempts(3), |e| {
        seen.push(e.clone());
        true
    });
    assert!(got.is_ok());
    assert_eq!(seen, vec![TransportError::Timeout, TransportError::Reset]);
}

#[test]
fn part4_single_attempt_never_consults_predicate() {
    let mut transport = Scripted::new(vec![Err(TransportError::Timeout)]);
    let mut calls = 0;
    let got = send_with_retry(&mut transport, &request(), attempts(1), |_| {
        calls += 1;
        true
    });
    assert_eq!(got, Err(TransportError::Timeout));
    assert_eq!(calls, 0, "with one attempt there is nothing to retry");
}

#[test]
fn part4_accepts_a_dyn_transport() {
    let mut transport = Scripted::new(vec![ok(204)]);
    let dyn_transport: &mut dyn Transport = &mut transport;
    let got = send_with_retry(dyn_transport, &request(), attempts(1), |_| false)
        .expect("succeeds through trait object");
    assert_eq!(got.status, 204);
}
