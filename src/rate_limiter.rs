//local shortcuts

//third-party shortcuts
use wasm_timer::Instant;

//standard shortcuts
use std::time::Duration;

//-------------------------------------------------------------------------------------------------------------------

/// Configuration for rate limiter. Defaults to 10 messages per 100 millisconds.
#[derive(Debug, Copy, Clone)]
pub struct RateLimitConfig
{
    /// Length of time to count messages. Defaults to 100 milliseconds.
    pub period: Duration,
    /// Max number of messages that may appear in a collection period. Defaults to 10 messages.
    pub max_count: u32
}

impl Default for RateLimitConfig
{
    fn default() -> RateLimitConfig
    {
        RateLimitConfig{
            period    : Duration::from_millis(100u64),
            max_count : 10u32,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Tracks and limits the rate that messages are accepted.
/// - If messages appear, on average, more frequently than count/period, then [`RateLimitTracker::try_count_msg()`]
///   will fail.
#[derive(Debug)]
pub struct RateLimitTracker
{
    /// rate limit configuration
    config: RateLimitConfig,
    /// timer
    timer: Instant,

    /// time of last checkpoint message (first message that appeared after end of last tracking period)
    next_checkpoint_time: Duration,
    /// number of messages received in this tracking period
    count: u64
}

impl RateLimitTracker
{
    /// Make a new rate limit tracker.
    pub fn new(config: RateLimitConfig) -> RateLimitTracker
    {
        let next_checkpoint_time = config.period;
        RateLimitTracker{
                config,
                timer: Instant::now(),
                next_checkpoint_time,
                count: 0u64
            }
    }

    /// Try to add a message to the tracker.
    /// - Fails if adding the message violates the rate limit.
    pub fn try_count_msg(&mut self) -> bool
    {
        // check if we are in a new period
        let msg_time = self.timer.elapsed();

        if msg_time >= self.next_checkpoint_time
        {
            // reset state for new tracking period
            self.next_checkpoint_time = msg_time.saturating_add(self.config.period);
            self.count = 0;
        }

        // increment count
        self.count += 1;

        // check if we have exceeded the rate limit
        if self.count > self.config.max_count as u64 { return false; }

        true
    }
}

//-------------------------------------------------------------------------------------------------------------------
