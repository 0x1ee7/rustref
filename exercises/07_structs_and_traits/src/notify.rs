//! Part 6: `Notifier` trait objects, injectable transports, and dispatch.

/// A boxed, thread-safe transport error.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Maximum SMS length in characters.
pub const SMS_MAX_CHARS: usize = 160;

/// Errors from building notifiers and sending messages.
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    /// The message is empty or whitespace only.
    #[error("message is blank")]
    BlankMessage,
    /// The recipient does not match the channel's format.
    #[error("invalid {channel} recipient {recipient:?}")]
    InvalidRecipient { channel: &'static str, recipient: String },
    /// The SMS is longer than the limit (in characters).
    #[error("message of {len} characters exceeds the limit of {max}")]
    MessageTooLong { len: usize, max: usize },
    /// A webhook endpoint that is not `https://`.
    #[error("webhook endpoint must use https: {0:?}")]
    InsecureEndpoint(String),
    /// The webhook recipient is empty or whitespace only.
    #[error("recipient is blank")]
    BlankRecipient,
    /// The endpoint answered with a status outside 200–299.
    #[error("endpoint returned HTTP status {0}")]
    HttpStatus(u16),
    /// The transport itself failed.
    #[error("transport failed")]
    Transport(#[source] BoxError),
    /// Two notifiers share a channel name.
    #[error("channel {0:?} is registered more than once")]
    DuplicateChannel(String),
    /// A requested channel is not registered.
    #[error("unknown channel {0:?}")]
    UnknownChannel(String),
}

/// A delivery channel. Object-safe, so dispatchers hold `Box<dyn Notifier>`.
pub trait Notifier {
    /// Unique channel name, for example `"email"`.
    fn channel_name(&self) -> &str;

    /// Validates and sends `message` to `recipient`.
    fn send(&self, _recipient: &str, _message: &str) -> Result<(), NotificationError>;

    /// Human-readable description. Defaults to `"<channel> notifier"`.
    fn describe(&self) -> String {
        todo!()
    }
}

/// Email channel. `transport(recipient, message)` does the actual delivery.
#[allow(dead_code)]
pub struct EmailNotifier<F> {
    transport: F,
}

impl<F> EmailNotifier<F> {
    /// Wraps a transport.
    pub fn new(_transport: F) -> Self {
        todo!()
    }
}

impl<F> Notifier for EmailNotifier<F>
where
    F: Fn(&str, &str) -> Result<(), BoxError>,
{
    fn channel_name(&self) -> &str {
        todo!()
    }

    fn send(&self, _recipient: &str, _message: &str) -> Result<(), NotificationError> {
        todo!()
    }
}

/// SMS channel. `transport(recipient, message)` does the actual delivery.
#[allow(dead_code)]
pub struct SmsNotifier<F> {
    transport: F,
}

impl<F> SmsNotifier<F> {
    /// Wraps a transport.
    pub fn new(_transport: F) -> Self {
        todo!()
    }
}

impl<F> Notifier for SmsNotifier<F>
where
    F: Fn(&str, &str) -> Result<(), BoxError>,
{
    fn channel_name(&self) -> &str {
        todo!()
    }

    fn send(&self, _recipient: &str, _message: &str) -> Result<(), NotificationError> {
        todo!()
    }
}

/// Webhook channel. `post(endpoint, body)` returns the HTTP status code.
#[allow(dead_code)]
pub struct WebhookNotifier<F> {
    endpoint: String,
    post: F,
}

impl<F> WebhookNotifier<F>
where
    F: Fn(&str, &[u8]) -> Result<u16, BoxError>,
{
    /// Creates a webhook channel. The endpoint must start with `https://`.
    pub fn new(_endpoint: &str, _post: F) -> Result<Self, NotificationError> {
        todo!()
    }
}

impl<F> Notifier for WebhookNotifier<F>
where
    F: Fn(&str, &[u8]) -> Result<u16, BoxError>,
{
    fn channel_name(&self) -> &str {
        todo!()
    }

    fn send(&self, _recipient: &str, _message: &str) -> Result<(), NotificationError> {
        todo!()
    }
}

/// Per-recipient outcome of [`NotificationDispatcher::dispatch`], in request order.
pub type DispatchOutcome<'a> = Vec<(&'a str, Result<(), NotificationError>)>;

/// Owns a set of channels with unique names.
pub struct NotificationDispatcher {
    #[allow(dead_code)]
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationDispatcher {
    /// Builds a dispatcher. Duplicate channel names are rejected.
    pub fn new(
        _notifiers: impl IntoIterator<Item = Box<dyn Notifier>>,
    ) -> Result<NotificationDispatcher, NotificationError> {
        todo!()
    }

    /// Channel names in construction order.
    pub fn channels(&self) -> impl Iterator<Item = &str> + '_ {
        #[allow(unreachable_code)]
        {
            todo!();
            std::iter::empty()
        }
    }

    /// Number of channels.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no channels are registered.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Sends `message` to each `(channel, recipient)` pair in order.
    ///
    /// Unknown channels fail before anything is sent. Per-send failures are
    /// returned in the result and do not stop the loop.
    pub fn dispatch<'a>(
        &self,
        _message: &str,
        _recipients: &[(&'a str, &str)],
    ) -> Result<DispatchOutcome<'a>, NotificationError> {
        todo!()
    }
}
