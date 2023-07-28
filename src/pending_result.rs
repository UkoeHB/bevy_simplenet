//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Handle for the result of async work (performed in a tokio runtime).
/// - The result will hang if the associated runtime is shut down.
#[derive(Debug)]
pub struct PendingResult<R>
{
    join_handle: Option<tokio::task::JoinHandle<R>>
}

impl<R> PendingResult<R>
{
    /// Make a new pending result.
    pub fn new(join_handle: tokio::task::JoinHandle<R>) -> PendingResult<R>
    {
        PendingResult{ join_handle: Some(join_handle) }
    }

    /// Make a pending result that is immediately ready.
    pub fn immediate(result: R, runtime: &tokio::runtime::Runtime) -> PendingResult<R>
    where
        R: Send + 'static
    {
        PendingResult::<R>::new(runtime.spawn(futures::future::ready::<R>(result)))
    }

    /// Check if result is available.
    pub fn has_result(&self) -> bool
    {
        match &self.join_handle
        {
            // has result if done running
            Some(handle) => handle.is_finished(),
            // result was already extracted
            None => false
        }
    }

    /// Check if work is done (result may be unavailable if already extracted).
    /// - This is robust for checking if a result-less task has completed (i.e. `PendingResult<()>`).
    pub fn is_done(&self) -> bool
    {
        if self.has_result() || self.join_handle.is_none() { return true; }
        false
    }

    /// Extract result if available (non-blocking).
    pub fn try_extract(&mut self) -> Option<Result<R, tokio::task::JoinError>>
    {
        // check if result available
        if !self.has_result() { return None; }

        // extract thread result
        let join_handle = self.join_handle.take().unwrap();
        Some(futures::executor::block_on(async move { join_handle.await } ))
    }

    /// Extract result if not yet extracted (blocking).
    pub fn extract(&mut self) -> Option<Result<R, tokio::task::JoinError>>
    {
        // check if already extracted
        if self.join_handle.is_none() { return None; }

        // extract thread result (blocks to wait)
        let join_handle = self.join_handle.take().unwrap();
        Some(futures::executor::block_on(async move { join_handle.await } ))
    }

    /// Extract result if not yet extracted (async).
    pub async fn extract_async(&mut self) -> Option<Result<R, tokio::task::JoinError>>
    {
        // check if already extracted
        if self.join_handle.is_none() { return None; }

        // extract thread result
        let join_handle = self.join_handle.take().unwrap();
        Some(join_handle.await)
    }
}

//-------------------------------------------------------------------------------------------------------------------
