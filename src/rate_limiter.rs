//local shortcuts

//third-party shortcuts

//standard shortcuts
use std::time::{Duration, Instant};

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Copy, Clone)]
pub struct RateLimitConfig
{
    /// Length of time to count messages.
    pub period: Duration,
    /// Max number of messages that may appear in a collection period.
    pub max_count: u32
}

//-------------------------------------------------------------------------------------------------------------------

/// Rate limit tracker.
/// - If messages appear, on average, more frequently than count/period, then [`RateLimitTracker::try_count_msg()`]
///   will fail.
#[derive(Debug)]
pub(crate) struct RateLimitTracker
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
    pub(crate) fn new(config: RateLimitConfig) -> RateLimitTracker
    {
        let next_checkpoint_time = config.period;
        RateLimitTracker{
                config,
                timer: Instant::now(),
                next_checkpoint_time,
                count: 1u64
            }
    }

    pub(crate) fn try_count_msg(&mut self) -> bool
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
