//! Part 6: lazy cursor pagination as an async stream, and an async resource pool with RAII leases.

mod support;

use std::future::{Ready, ready};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ex15_async_tokio::error::ConfigError;
use ex15_async_tokio::resources::{Factory, LeaseError, Page, Paginator, ResourcePool};

const ONE_SEC: Duration = Duration::from_secs(1);

type Seen = Arc<Mutex<Vec<Option<String>>>>;

fn page(items: Vec<u32>, next: Option<&str>) -> Result<Page<u32>, String> {
    Ok(Page {
        items,
        next_cursor: next.map(str::to_string),
    })
}

/// A fake API that replays `script` one response per request, recording the cursors it was given.
fn scripted(
    script: Vec<Result<Page<u32>, String>>,
    seen: Seen,
) -> impl FnMut(Option<String>) -> Ready<Result<Page<u32>, String>> {
    let mut script = script.into_iter();
    move |cursor| {
        seen.lock().unwrap().push(cursor);
        ready(script.next().unwrap_or_else(|| Err("script exhausted".to_string())))
    }
}

fn three_pages(seen: Seen) -> impl FnMut(Option<String>) -> Ready<Result<Page<u32>, String>> {
    scripted(
        vec![
            page(vec![1, 2], Some("c1")),
            page(vec![], Some("c2")),
            page(vec![3], None),
        ],
        seen,
    )
}

async fn drain<F, E>(paginator: &mut Paginator<u32, E, F>) -> Vec<Result<u32, E>>
where
    F: FnMut(Option<String>) -> Ready<Result<Page<u32>, E>>,
    E: Send,
{
    let mut out = Vec::new();
    while let Some(item) = paginator.next().await {
        out.push(item);
    }
    out
}

#[derive(Clone, Default)]
struct CountingFactory {
    created: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    fail_next: Arc<AtomicBool>,
}

impl Factory for CountingFactory {
    type Resource = u32;
    type Error = String;

    fn create(&self) -> impl Future<Output = Result<u32, String>> + Send {
        let fail = self.fail_next.swap(false, Ordering::SeqCst);
        let created = self.created.clone();
        async move {
            if fail {
                Err("connect refused".to_string())
            } else {
                let id = created.fetch_add(1, Ordering::SeqCst);
                Ok(u32::try_from(id).expect("small test id"))
            }
        }
    }

    fn close(&self, _resource: u32) -> impl Future<Output = ()> + Send {
        let closed = self.closed.clone();
        async move {
            closed.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn part6_zero_limits_are_rejected() {
    let paginator = Paginator::<u32, String, _>::new(
        scripted(Vec::new(), Seen::default()),
        Some(0),
    );
    assert!(matches!(paginator, Err(ConfigError::ZeroMaxPages)));

    let pool = ResourcePool::new(CountingFactory::default(), 0);
    assert!(matches!(pool, Err(ConfigError::ZeroLimit)));
}

#[tokio::test(start_paused = true)]
async fn part6_walks_every_page_in_order_including_empty_pages() {
    let seen = Seen::default();
    let mut paginator = Paginator::new(three_pages(seen.clone()), None).expect("valid limit");

    let items: Vec<u32> = drain(&mut paginator)
        .await
        .into_iter()
        .map(|item| item.expect("no errors scripted"))
        .collect();

    assert_eq!(items, vec![1, 2, 3], "an empty page must not end the walk");
    assert_eq!(paginator.pages_fetched(), 3);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![None, Some("c1".to_string()), Some("c2".to_string())],
        "the first request has no cursor; each later one carries the previous next_cursor"
    );
}

#[tokio::test(start_paused = true)]
async fn part6_fetches_lazily_one_page_at_a_time() {
    let mut paginator =
        Paginator::new(three_pages(Seen::default()), None).expect("valid limit");

    assert_eq!(paginator.next().await, Some(Ok(1)));
    assert_eq!(
        paginator.pages_fetched(),
        1,
        "the second page must not be requested until its items are needed"
    );
}

#[tokio::test(start_paused = true)]
async fn part6_max_pages_caps_the_number_of_requests() {
    let script = vec![
        page(vec![1, 2], Some("c1")),
        page(vec![3, 4], Some("c2")),
        page(vec![5], Some("c3")),
    ];
    let mut paginator =
        Paginator::new(scripted(script, Seen::default()), Some(2)).expect("valid limit");

    let items: Vec<u32> = drain(&mut paginator)
        .await
        .into_iter()
        .map(|item| item.expect("no errors scripted"))
        .collect();

    assert_eq!(items, vec![1, 2, 3, 4], "the third page is never requested");
    assert_eq!(paginator.pages_fetched(), 2);
}

#[tokio::test(start_paused = true)]
async fn part6_fetch_error_ends_the_stream_and_is_sticky() {
    let script = vec![page(vec![1], Some("c1")), Err("throttled".to_string())];
    let mut paginator =
        Paginator::new(scripted(script, Seen::default()), None).expect("valid limit");

    assert_eq!(paginator.next().await, Some(Ok(1)));
    assert_eq!(paginator.next().await, Some(Err("throttled".to_string())));
    assert_eq!(paginator.next().await, None, "after an error the stream is finished");
    assert_eq!(paginator.next().await, None);
    assert_eq!(
        paginator.pages_fetched(),
        2,
        "the failed request counts; no further requests are made after it"
    );
}

#[tokio::test(start_paused = true)]
async fn part6_lease_reuses_an_idle_resource() {
    let factory = CountingFactory::default();
    let pool = ResourcePool::new(factory, 2).expect("valid size");

    let first = pool.lease().await.expect("first lease");
    assert_eq!(*first, 0);
    drop(first);

    let second = pool.lease().await.expect("second lease");
    assert_eq!(*second, 0, "the idle resource is handed out again");
    assert_eq!(pool.created(), 1, "no new resource is created");
    assert_eq!(pool.idle(), 0, "the resource is leased, not idle");
}

#[tokio::test(start_paused = true)]
async fn part6_lease_waits_for_a_free_slot() {
    let pool = Arc::new(ResourcePool::new(CountingFactory::default(), 1).expect("valid size"));
    let held = pool.lease().await.expect("first lease");

    let waiter = {
        let pool = pool.clone();
        tokio::spawn(async move { pool.lease().await.map(|lease| *lease) })
    };
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(!waiter.is_finished(), "a second lease must wait while the only slot is held");

    drop(held);
    let value = tokio::time::timeout(ONE_SEC, waiter)
        .await
        .expect("releasing the lease wakes the waiter")
        .expect("waiter task must not panic")
        .expect("lease succeeds");
    assert_eq!(value, 0);
    assert_eq!(pool.created(), 1, "the released resource is handed over, not recreated");
}

#[tokio::test(start_paused = true)]
async fn part6_cancelled_waiter_does_not_consume_a_slot() {
    let pool = Arc::new(ResourcePool::new(CountingFactory::default(), 1).expect("valid size"));
    let held = pool.lease().await.expect("first lease");

    let waiter = {
        let pool = pool.clone();
        tokio::spawn(async move { pool.lease().await.map(|lease| *lease) })
    };
    tokio::time::sleep(Duration::from_millis(10)).await;
    waiter.abort();
    let _ = waiter.await;

    drop(held);
    let lease = tokio::time::timeout(ONE_SEC, pool.lease())
        .await
        .expect("the slot is free once the holder releases it")
        .expect("lease succeeds");
    assert_eq!(*lease, 0);
}

#[tokio::test(start_paused = true)]
async fn part6_factory_error_is_reported_and_frees_the_slot() {
    let factory = CountingFactory::default();
    factory.fail_next.store(true, Ordering::SeqCst);
    let pool = ResourcePool::new(factory, 1).expect("valid size");

    match pool.lease().await {
        Err(LeaseError::Factory(message)) => assert_eq!(message, "connect refused"),
        Err(LeaseError::Closed) => panic!("the pool is open; expected a factory error"),
        Ok(_) => panic!("the factory was told to fail"),
    }
    assert_eq!(pool.created(), 0, "a failed creation does not count as created");

    let lease = tokio::time::timeout(ONE_SEC, pool.lease())
        .await
        .expect("the slot must be released after a failed create")
        .expect("second lease succeeds");
    assert_eq!(*lease, 0);
}

#[tokio::test(start_paused = true)]
async fn part6_lease_is_released_when_its_holder_panics() {
    let pool = Arc::new(ResourcePool::new(CountingFactory::default(), 1).expect("valid size"));

    let crashed = {
        let pool = pool.clone();
        tokio::spawn(async move {
            let _lease = pool.lease().await.expect("first lease");
            panic!("worker crashed while holding a resource");
        })
    };
    assert!(crashed.await.is_err(), "the holder task panicked");

    let lease = tokio::time::timeout(ONE_SEC, pool.lease())
        .await
        .expect("unwinding must release the lease")
        .expect("lease succeeds");
    assert_eq!(*lease, 0);
}

#[tokio::test(start_paused = true)]
async fn part6_close_closes_idle_defers_leased_and_rejects_new_leases() {
    let factory = CountingFactory::default();
    let pool = ResourcePool::new(factory.clone(), 2).expect("valid size");

    let a = pool.lease().await.expect("lease a");
    let b = pool.lease().await.expect("lease b");
    drop(a);
    assert_eq!(pool.idle(), 1);

    pool.close().await;
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(
        factory.closed.load(Ordering::SeqCst),
        1,
        "the idle resource is closed by close()"
    );

    assert!(
        matches!(pool.lease().await, Err(LeaseError::Closed)),
        "leasing from a closed pool must fail"
    );

    drop(b);
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(
        factory.closed.load(Ordering::SeqCst),
        2,
        "the leased resource is closed when its lease is dropped after close"
    );
}
