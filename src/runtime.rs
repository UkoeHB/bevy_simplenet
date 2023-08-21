//local shortcuts

//third-party shortcuts

//standard shortcuts
use std::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

pub trait OneshotRuntime: Debug + Send + 'static
{
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()>,
        F: Send + 'static;
}

//-------------------------------------------------------------------------------------------------------------------

pub trait SimpleRuntime<R>: Debug + Send + 'static
{
    type Error;
    type Future: std::future::Future<Output = Result<R, Self::Error>> + Debug + Send + 'static;

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
