//! Part 6: `Notifier` trait objects, injectable transports, and dispatch.

use serde::Serialize;

use crate::employee::is_plausible_email;

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
    InvalidRecipient {
        channel: &'static str,
        recipient: String,
    },
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
    fn send(&self, recipient: &str, message: &str) -> Result<(), NotificationError>;

    /// Human-readable description. Defaults to `"<channel> notifier"`.
    fn describe(&self) -> String {
        format!("{} notifier", self.channel_name())
    }
}

/// Rejects empty and whitespace-only messages.
fn ensure_message(message: &str) -> Result<(), NotificationError> {
    if message.trim().is_empty() {
        Err(NotificationError::BlankMessage)
    } else {
        Ok(())
    }
}

/// Email channel. `transport(recipient, message)` does the actual delivery.
pub struct EmailNotifier<F> {
    transport: F,
}

impl<F> EmailNotifier<F> {
    /// Wraps a transport.
    pub fn new(transport: F) -> Self {
        EmailNotifier { transport }
    }
}

impl<F> Notifier for EmailNotifier<F>
where
    F: Fn(&str, &str) -> Result<(), BoxError>,
{
    fn channel_name(&self) -> &str {
        "email"
    }

    fn send(&self, recipient: &str, message: &str) -> Result<(), NotificationError> {
        ensure_message(message)?;
        if !is_plausible_email(recipient) {
            return Err(NotificationError::InvalidRecipient {
                channel: "email",
                recipient: recipient.to_owned(),
            });
        }
        (self.transport)(recipient, message).map_err(NotificationError::Transport)
    }
}

/// SMS channel. `transport(recipient, message)` does the actual delivery.
pub struct SmsNotifier<F> {
    transport: F,
}

impl<F> SmsNotifier<F> {
    /// Wraps a transport.
    pub fn new(transport: F) -> Self {
        SmsNotifier { transport }
    }
}

/// `true` for `+` followed by 7–15 digits, the first of which is not `0`.
fn is_e164ish(recipient: &str) -> bool {
    let Some(digits) = recipient.strip_prefix('+') else {
        return false;
    };
    (7..=15).contains(&digits.len())
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.starts_with('0')
}

impl<F> Notifier for SmsNotifier<F>
where
    F: Fn(&str, &str) -> Result<(), BoxError>,
{
    fn channel_name(&self) -> &str {
        "sms"
    }

    fn send(&self, recipient: &str, message: &str) -> Result<(), NotificationError> {
        ensure_message(message)?;
        if !is_e164ish(recipient) {
            return Err(NotificationError::InvalidRecipient {
                channel: "sms",
                recipient: recipient.to_owned(),
            });
        }
        let len = message.chars().count();
        if len > SMS_MAX_CHARS {
            return Err(NotificationError::MessageTooLong {
                len,
                max: SMS_MAX_CHARS,
            });
        }
        (self.transport)(recipient, message).map_err(NotificationError::Transport)
    }
}

/// Webhook channel. `post(endpoint, body)` returns the HTTP status code.
pub struct WebhookNotifier<F> {
    endpoint: String,
    post: F,
}

impl<F> WebhookNotifier<F>
where
    F: Fn(&str, &[u8]) -> Result<u16, BoxError>,
{
    /// Creates a webhook channel. The endpoint must start with `https://`.
    pub fn new(endpoint: &str, post: F) -> Result<Self, NotificationError> {
        if !endpoint.starts_with("https://") {
            return Err(NotificationError::InsecureEndpoint(endpoint.to_owned()));
        }
        Ok(WebhookNotifier {
            endpoint: endpoint.to_owned(),
            post,
        })
    }
}

/// JSON body of a webhook call. Field order is the wire order.
#[derive(Serialize)]
struct WebhookBody<'a> {
    message: &'a str,
    recipient: &'a str,
}

impl<F> Notifier for WebhookNotifier<F>
where
    F: Fn(&str, &[u8]) -> Result<u16, BoxError>,
{
    fn channel_name(&self) -> &str {
        "webhook"
    }

    fn send(&self, recipient: &str, message: &str) -> Result<(), NotificationError> {
        ensure_message(message)?;
        if recipient.trim().is_empty() {
            return Err(NotificationError::BlankRecipient);
        }
        // Serialising two string fields cannot fail.
        let body = serde_json::to_vec(&WebhookBody { message, recipient })
            .expect("serialising string fields cannot fail");
        let status = (self.post)(self.endpoint.as_str(), &body)
            .map_err(NotificationError::Transport)?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(NotificationError::HttpStatus(status))
        }
    }
}

/// Per-recipient outcome of [`NotificationDispatcher::dispatch`], in request order.
pub type DispatchOutcome<'a> = Vec<(&'a str, Result<(), NotificationError>)>;

/// Owns a set of channels with unique names.
pub struct NotificationDispatcher {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationDispatcher {
    /// Builds a dispatcher. Duplicate channel names are rejected.
    pub fn new(
        notifiers: impl IntoIterator<Item = Box<dyn Notifier>>,
    ) -> Result<NotificationDispatcher, NotificationError> {
        let mut registered: Vec<Box<dyn Notifier>> = Vec::new();
        for notifier in notifiers {
            if registered
                .iter()
                .any(|existing| existing.channel_name() == notifier.channel_name())
            {
                return Err(NotificationError::DuplicateChannel(
                    notifier.channel_name().to_owned(),
                ));
            }
            registered.push(notifier);
        }
        Ok(NotificationDispatcher {
            notifiers: registered,
        })
    }

    /// Channel names in construction order.
    pub fn channels(&self) -> impl Iterator<Item = &str> + '_ {
        self.notifiers.iter().map(|notifier| notifier.channel_name())
    }

    /// Number of channels.
    pub fn len(&self) -> usize {
        self.notifiers.len()
    }

    /// `true` when no channels are registered.
    pub fn is_empty(&self) -> bool {
        self.notifiers.is_empty()
    }

    /// Sends `message` to each `(channel, recipient)` pair in order.
    ///
    /// Unknown channels fail before anything is sent. Per-send failures are
    /// returned in the result and do not stop the loop.
    pub fn dispatch<'a>(
        &self,
        message: &str,
        recipients: &[(&'a str, &str)],
    ) -> Result<DispatchOutcome<'a>, NotificationError> {
        // Resolve every channel first so an unknown one aborts before any send.
        let mut targets: Vec<(&'a str, &dyn Notifier, &str)> = Vec::with_capacity(recipients.len());
        for &(channel, recipient) in recipients {
            let notifier = self
                .notifiers
                .iter()
                .find(|notifier| notifier.channel_name() == channel)
                .ok_or_else(|| NotificationError::UnknownChannel(channel.to_owned()))?;
            targets.push((channel, notifier.as_ref(), recipient));
        }
        Ok(targets
            .into_iter()
            .map(|(channel, notifier, recipient)| (channel, notifier.send(recipient, message)))
            .collect())
    }
}
