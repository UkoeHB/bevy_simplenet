//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

pub trait OneshotRuntime: Send + 'static
{
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()>,
        F: Send + 'static;
}

//-------------------------------------------------------------------------------------------------------------------

pub trait SimpleRuntime<R>: Send + 'static
{
    type Error;
    type Future: std::future::Future<Output = Result<R, Self::Error>> + Send + 'static;

    fn spawn<F>(&self, task: F) -> Self::Future
    where
        F: std::future::Future<Output = R>,
        F: Send + 'static;

    fn is_terminated(f: &Self::Future) -> bool;
}

impl<SRt: SimpleRuntime::<()>> OneshotRuntime for SRt
{
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()>,
        F: Send + 'static
    {
        self.spawn(task);  //discard future (assume oneshots use a oneshot channel)
    }
}

//-------------------------------------------------------------------------------------------------------------------
