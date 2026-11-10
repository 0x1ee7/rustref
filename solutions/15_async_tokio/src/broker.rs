//! Part 4 — publish/subscribe on a `broadcast` channel with an explicit lagging-subscriber policy.

use tokio::sync::broadcast::{self, error::RecvError};

use crate::error::ConfigError;

/// What a subscriber receives from [`Subscription::next`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery<M> {
    /// The next message, in publication order.
    Message(M),
    /// The subscriber fell behind; this many messages were overwritten before it read them.
    Lagged(u64),
    /// The broker was dropped and every buffered message has been delivered.
    Closed,
}

/// Fan-out broker. Publishing never waits for subscribers.
///
/// Subscribers that fall more than `capacity` messages behind are told how many messages they
/// missed (`Delivery::Lagged`) and then continue from the oldest message still buffered.
pub struct Broker<M> {
    sender: broadcast::Sender<M>,
}

impl<M: Clone> Broker<M> {
    /// Creates a broker that buffers up to `capacity` messages per subscriber.
    /// `capacity == 0` is `ConfigError::ZeroBuffer`.
    pub fn new(capacity: usize) -> Result<Self, ConfigError> {
        if capacity == 0 {
            return Err(ConfigError::ZeroBuffer);
        }
        let (sender, _receiver) = broadcast::channel(capacity);
        Ok(Self { sender })
    }

    /// Publishes to every current subscriber and returns how many received it.
    /// With no subscribers the message is discarded and `0` is returned.
    pub fn publish(&self, message: M) -> usize {
        self.sender.send(message).unwrap_or(0)
    }

    /// Creates a subscription that sees messages published from now on.
    pub fn subscribe(&self) -> Subscription<M> {
        Subscription {
            receiver: self.sender.subscribe(),
        }
    }

    /// Number of live subscriptions.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// A single subscriber's view of a [`Broker`]. Dropping it unsubscribes.
pub struct Subscription<M> {
    receiver: broadcast::Receiver<M>,
}

impl<M: Clone> Subscription<M> {
    /// Waits for the next delivery.
    pub async fn next(&mut self) -> Delivery<M> {
        match self.receiver.recv().await {
            Ok(message) => Delivery::Message(message),
            Err(RecvError::Lagged(missed)) => Delivery::Lagged(missed),
            Err(RecvError::Closed) => Delivery::Closed,
        }
    }
}
