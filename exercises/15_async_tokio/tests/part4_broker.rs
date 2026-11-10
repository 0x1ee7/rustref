//! Part 4: publish/subscribe with an explicit lagging-subscriber policy.

mod support;

use std::time::Duration;

use ex15_async_tokio::broker::{Broker, Delivery};
use ex15_async_tokio::error::ConfigError;

const ONE_SEC: Duration = Duration::from_secs(1);

#[tokio::test(start_paused = true)]
async fn part4_zero_buffer_is_rejected() {
    assert!(matches!(Broker::<u32>::new(0), Err(ConfigError::ZeroBuffer)));
}

#[tokio::test(start_paused = true)]
async fn part4_publish_without_subscribers_discards_and_returns_zero() {
    let broker = Broker::new(4).expect("valid capacity");

    assert_eq!(broker.publish(1u32), 0, "no subscriber received the message");
    assert_eq!(broker.subscriber_count(), 0);
}

#[tokio::test(start_paused = true)]
async fn part4_every_subscriber_receives_every_message_in_order() {
    let broker = Broker::new(8).expect("valid capacity");
    let mut first = broker.subscribe();
    let mut second = broker.subscribe();
    assert_eq!(broker.subscriber_count(), 2);

    assert_eq!(broker.publish(1u32), 2, "publish reports the number of receivers");
    assert_eq!(broker.publish(2), 2);

    for sub in [&mut first, &mut second] {
        assert_eq!(sub.next().await, Delivery::Message(1));
        assert_eq!(sub.next().await, Delivery::Message(2));
    }
}

#[tokio::test(start_paused = true)]
async fn part4_late_subscriber_only_sees_messages_published_after_it_subscribed() {
    let broker = Broker::new(8).expect("valid capacity");
    broker.publish(1u32);

    let mut late = broker.subscribe();
    broker.publish(2);

    assert_eq!(late.next().await, Delivery::Message(2));
}

#[tokio::test(start_paused = true)]
async fn part4_slow_subscriber_is_told_how_many_messages_it_missed() {
    let broker = Broker::new(2).expect("valid capacity");
    let mut slow = broker.subscribe();
    for n in 1..=5u32 {
        broker.publish(n);
    }

    assert_eq!(
        slow.next().await,
        Delivery::Lagged(3),
        "five messages into a buffer of two: three were overwritten"
    );
    assert_eq!(slow.next().await, Delivery::Message(4), "continues from the newest kept message");
    assert_eq!(slow.next().await, Delivery::Message(5));
}

#[tokio::test(start_paused = true)]
async fn part4_publish_never_waits_for_a_stalled_subscriber() {
    let broker = Broker::new(1).expect("valid capacity");
    let _stalled = broker.subscribe();

    tokio::time::timeout(ONE_SEC, async {
        for n in 0..100u32 {
            assert_eq!(broker.publish(n), 1, "the stalled subscriber still counts as a receiver");
        }
    })
    .await
    .expect("publishing must never block on a subscriber");
}

#[tokio::test(start_paused = true)]
async fn part4_dropping_the_broker_delivers_buffered_messages_then_closed() {
    let broker = Broker::new(4).expect("valid capacity");
    let mut sub = broker.subscribe();
    broker.publish(1u32);
    broker.publish(2);
    drop(broker);

    assert_eq!(sub.next().await, Delivery::Message(1));
    assert_eq!(sub.next().await, Delivery::Message(2));
    assert_eq!(sub.next().await, Delivery::Closed);
}

#[tokio::test(start_paused = true)]
async fn part4_dropping_a_subscription_unsubscribes_it() {
    let broker = Broker::<u32>::new(4).expect("valid capacity");
    let first = broker.subscribe();
    let second = broker.subscribe();
    assert_eq!(broker.subscriber_count(), 2);

    drop(first);
    assert_eq!(broker.subscriber_count(), 1);
    assert_eq!(broker.publish(1), 1);

    drop(second);
    assert_eq!(broker.subscriber_count(), 0);
}
