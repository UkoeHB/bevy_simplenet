//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use std::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct TokioRuntime<R>
{
    handle: tokio::runtime::Handle,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Debug + Send + 'static> SimpleRuntime<R> for TokioRuntime<R>
{
    type Error = tokio::task::JoinError;
    type Future = tokio::task::JoinHandle<R>;

    fn spawn<F>(&self, task: F) -> Self::Future
    where
        F: std::future::Future<Output = R>,
        F: Send + 'static,
    {
        self.handle.spawn(task)
    }

    fn is_terminated(f: &Self::Future) -> bool
    {
        f.is_finished()
    }
}

impl<R: Send + 'static> From<tokio::runtime::Runtime> for TokioRuntime<R>
{
    fn from(runtime: tokio::runtime::Runtime) -> Self
    {
        Self::from(&runtime)
    }
}

impl<R: Send + 'static> From<&tokio::runtime::Runtime> for TokioRuntime<R>
{
    fn from(runtime: &tokio::runtime::Runtime) -> Self
    {
        TokioRuntime::<R>{ handle: runtime.handle().clone(), _phantom: std::marker::PhantomData::<R>::default() }
    }
}

impl<R: Send + 'static> From<tokio::runtime::Handle> for TokioRuntime<R>
{
    fn from(handle: tokio::runtime::Handle) -> Self
    {
        TokioRuntime::<R>{ handle, _phantom: std::marker::PhantomData::<R>::default() }
    }
}

impl<R: Send + 'static> From<&tokio::runtime::Handle> for TokioRuntime<R>
{
    fn from(handle: &tokio::runtime::Handle) -> Self
    {
        Self::from(handle.clone())
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct StdRuntime;

impl OneshotRuntime for StdRuntime
{
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()>,
        F: Send + 'static,
    {
        std::thread::spawn(move || futures::executor::block_on(async move { task.await }));
    }
}

impl From<EmptyRuntime> for StdRuntime
{
    fn from(_: EmptyRuntime) -> Self
    {
        StdRuntime{}
    }
}

impl From<EmptyRuntime> for &StdRuntime
{
    fn from(_: EmptyRuntime) -> Self
    {
        &StdRuntime{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
