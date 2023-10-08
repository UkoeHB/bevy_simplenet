//local shortcuts

//third-party shortcuts

//standard shortcuts
use std::time::{Duration, SystemTime, UNIX_EPOCH};

//-------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn text_ping_fn(timestamp: std::time::Duration) -> ezsockets::RawMessage
{
    let timestamp = timestamp.as_millis();
    ezsockets::RawMessage::Text(format!("ping:{}", timestamp))
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) fn log_ping_pong_latency(timestamp: u128)
{
    let timestamp = Duration::from_millis(timestamp as u64); // TODO: handle overflow
    let latency = SystemTime::now()
        .duration_since(UNIX_EPOCH + timestamp)
        .unwrap_or(Duration::default());
    tracing::trace!("latency: {}ms", latency.as_millis());
}

//-------------------------------------------------------------------------------------------------------------------
