//! Part 5: middleware chains built from boxed closures.

use ex04_closures_and_functions::middleware::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn tracing(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
) -> impl Fn(Request, &dyn Fn(Request) -> Response) -> Response + Send + Sync + 'static {
    move |request: Request, next: &dyn Fn(Request) -> Response| {
        log.lock().unwrap().push(format!("{name}>"));
        let response = next(request);
        log.lock().unwrap().push(format!("<{name}"));
        response
    }
}

#[test]
fn part5_empty_pipeline_calls_the_endpoint_directly() {
    let pipeline = Pipeline::new(|req: Request| Response::new(200, format!("hit {}", req.path)));
    let response = pipeline.handle(Request::new("/ping"));
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "hit /ping");
}

#[test]
fn part5_first_added_layer_is_outermost() {
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let endpoint_log = Arc::clone(&log);
    let pipeline = Pipeline::new(move |_req: Request| {
        endpoint_log.lock().unwrap().push("endpoint".to_string());
        Response::new(200, "ok")
    })
    .layer(tracing("A", Arc::clone(&log)))
    .layer(tracing("B", Arc::clone(&log)));

    pipeline.handle(Request::new("/"));
    assert_eq!(*log.lock().unwrap(), ["A>", "B>", "endpoint", "<B", "<A"]);
}

#[test]
fn part5_require_header_short_circuits_without_calling_endpoint() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let pipeline = Pipeline::new(move |_req: Request| {
        counter.fetch_add(1, Ordering::SeqCst);
        Response::new(200, "secret")
    })
    .layer(require_header("x-api-key"));

    let response = pipeline.handle(Request::new("/data"));
    assert_eq!(response.status, 400);
    assert!(response.body.contains("x-api-key"), "the body names the missing header");
    assert_eq!(hits.load(Ordering::SeqCst), 0, "the endpoint must not run");
}

#[test]
fn part5_header_names_are_case_insensitive() {
    let request = Request::new("/data").with_header("X-API-Key", "k1");
    assert_eq!(request.header("x-api-key"), Some("k1"));

    let pipeline =
        Pipeline::new(|_req: Request| Response::new(200, "secret")).layer(require_header("X-API-KEY"));
    assert_eq!(pipeline.handle(request).status, 200);
}

#[test]
fn part5_with_header_replaces_an_existing_value() {
    let request = Request::new("/").with_header("A", "1").with_header("a", "2");
    assert_eq!(request.header("A"), Some("2"), "the later value wins, regardless of case");
}

#[test]
fn part5_layers_can_rewrite_the_request_and_the_response() {
    let pipeline = Pipeline::new(|req: Request| {
        let id = req.header("x-request-id").unwrap_or("none").to_string();
        Response::new(200, id)
    })
    .layer(|req: Request, next: &dyn Fn(Request) -> Response| {
        let response = next(req.with_header("x-request-id", "abc-123"));
        response.with_header("x-served-by", "edge-1")
    });

    let response = pipeline.handle(Request::new("/"));
    assert_eq!(response.body, "abc-123", "the inner endpoint sees the rewritten request");
    assert_eq!(response.header("x-served-by"), Some("edge-1"));
}

#[test]
fn part5_pipeline_is_shared_across_threads() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let pipeline = Arc::new(
        Pipeline::new(move |_req: Request| {
            counter.fetch_add(1, Ordering::SeqCst);
            Response::new(200, "")
        })
        .layer(require_header("x-api-key")),
    );

    let workers: Vec<_> = (0..4)
        .map(|_| {
            let pipeline = Arc::clone(&pipeline);
            thread::spawn(move || {
                (0..100)
                    .filter(|_| {
                        let request = Request::new("/").with_header("x-api-key", "k");
                        pipeline.handle(request).status == 200
                    })
                    .count()
            })
        })
        .collect();
    let served: usize = workers
        .into_iter()
        .map(|w| w.join().expect("worker panicked"))
        .sum();
    assert_eq!(served, 400);
    assert_eq!(hits.load(Ordering::SeqCst), 400);
}
