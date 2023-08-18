//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Handle for the result of async work (performed in a std::thread).
#[derive(Debug)]
pub struct StdPendingResult<R>
{
    join_handle: Option<std::thread::JoinHandle<R>>
}

impl<R> StdPendingResult<R>
{
    /// Make a new pending result.
    pub fn new(join_handle: std::thread::JoinHandle<R>) -> StdPendingResult<R>
    {
        StdPendingResult{ join_handle: Some(join_handle) }
    }

    /// Make a pending result that is immediately ready.
    pub fn immediate(result: R) -> StdPendingResult<R>
    where
        R: Send + 'static
    {
        StdPendingResult::<R>::new(std::thread::spawn(|| result))
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
    /// - This is robust for checking if a result-less task has completed (i.e. `StdPendingResult<()>`).
    pub fn is_done(&self) -> bool
    {
        if self.has_result() || self.join_handle.is_none() { return true; }
        false
    }

    /// Extract result if available (non-blocking).
    pub fn try_extract(&mut self) -> Option<Result<R, ()>>
    {
        // check if result available
        if !self.has_result() { return None; }

        // extract thread result
        let join_handle = self.join_handle.take().unwrap();
        Some(join_handle.join().map_err(|_| ()))
    }

    /// Extract result if not yet extracted (blocking).
    pub fn extract(&mut self) -> Option<Result<R, ()>>
    {
        // check if already extracted
        if self.join_handle.is_none() { return None; }

        // extract thread result (blocks to wait)
        let join_handle = self.join_handle.take().unwrap();
        Some(join_handle.join().map_err(|_| ()))
    }
}

//-------------------------------------------------------------------------------------------------------------------
