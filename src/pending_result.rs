//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Result of a 'pending result'.
pub enum PRResult<R>
{
    /// Result has already been taken.
    Taken,
    /// Result is 'error'.
    Error,
    /// The successful result.
    Result(R),
}

//-------------------------------------------------------------------------------------------------------------------

/// The pending result of async work.
#[derive(Debug)]
pub struct PendingResult<Recv: ResultReceiver>
{
    result_receiver: Option<Recv>,
}

impl<Recv: ResultReceiver> PendingResult<Recv>
{
    /// Make a new pending result.
    pub fn new<T, F>(task: T, runtime: &Recv::Runtime) -> Self
    where
        T: FnOnce() -> F + Send + 'static,
        F: std::future::Future<Output = Recv::Result> + Send + 'static,
    {
        let result_receiver = Recv::new(task, runtime);
        Self{ result_receiver: Some(result_receiver) }
    }

    /// Make a pending result that is immediately ready.
    pub fn immediate(result: Recv::Result, runtime: &Recv::Runtime) -> Self
    {
        let result_receiver = Recv::immediate(result, runtime);
        Self{ result_receiver: Some(result_receiver) }
    }

    /// Check if result is available.
    pub fn has_result(&self) -> bool
    {
        match &self.result_receiver
        {
            // has result if done running
            Some(receiver) => receiver.done(),
            // result was already extracted
            None => false
        }
    }

    /// Check if work is done (result may be unavailable if already extracted).
    /// - This is robust for checking if a result-less task has completed (i.e. `PendingResult<()>`).
    pub fn is_done(&self) -> bool
    {
        if self.has_result() || self.result_receiver.is_none() { return true; }
        false
    }

    /// Extract result if available (non-blocking).
    /// Returns `None` if the result is still pending.
    pub fn try_extract(&mut self) -> Option<PRResult<Recv::Result>>
    {
        // check if result is pending
        if !self.has_result() && !self.result_receiver.is_none() { return None; }

        // extract thread result
        Some(self.extract())
    }

    /// Extract result (blocking).
    pub fn extract(&mut self) -> PRResult<Recv::Result>
    {
        futures::executor::block_on(async || self.extract_async().await)
    }

    /// Extract result (async).
    pub async fn extract_async(&mut self) -> PRResult<Recv::Result>
    {
        // consume the result receiver
        let Some(receiver) = self.result_receiver.take() else { return PRResult::Taken; };

        // await thread result
        let Some(res) = receiver.get().await else { return PRResult::Error; };

        PRResult::Result(res)
    }
}

//-------------------------------------------------------------------------------------------------------------------
