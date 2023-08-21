//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use std::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Implements `SimpleRuntime` for `tokio` runtimes (spawn on tokio runtime).
/// If no other type implements `DefaultIORuntime`, this is the default IO runtime on native builds.
#[derive(Debug)]
pub struct TokioRuntimeImpl<R>
{
    handle: tokio::runtime::Handle,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Debug + Send + 'static> SimpleRuntime<R> for TokioRuntimeImpl<R>
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

impl<R: Send + 'static> From<tokio::runtime::Runtime> for TokioRuntimeImpl<R> {
    fn from(runtime: tokio::runtime::Runtime) -> Self {
        Self::from(&runtime)
    }
}

impl<R: Send + 'static> From<&tokio::runtime::Runtime> for TokioRuntimeImpl<R> {
    fn from(runtime: &tokio::runtime::Runtime) -> Self {
        TokioRuntimeImpl::<R>{ handle: runtime.handle().clone(), _phantom: std::marker::PhantomData::<R>::default() }
    }
}

impl<R: Send + 'static> From<tokio::runtime::Handle> for TokioRuntimeImpl<R> {
    fn from(handle: tokio::runtime::Handle) -> Self {
        TokioRuntimeImpl::<R>{ handle, _phantom: std::marker::PhantomData::<R>::default() }
    }
}

impl<R: Send + 'static> From<&tokio::runtime::Handle> for TokioRuntimeImpl<R> {
    fn from(handle: &tokio::runtime::Handle) -> Self {
        Self::from(handle.clone())
    }
}

impl<R: Send + 'static> From<DefaultIORuntime> for TokioRuntimeImpl<R> {
    fn from(handle: DefaultIORuntime) -> Self {
        TokioRuntimeImpl::<R>::from(tokio::runtime::Handle::from(handle))
    }
}

impl<R: Send + 'static> From<&DefaultIORuntime> for TokioRuntimeImpl<R> {
    fn from(handle: &DefaultIORuntime) -> Self {
        Self::from(handle.clone())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Implements `OneshotRuntime` for `std` runtimes (spawn new thread).
/// If no other type implements `DefaultCPURuntime`, this is the default CPU runtime on native builds.
#[derive(Debug)]
pub struct StdRuntimeImpl;

impl OneshotRuntime for StdRuntimeImpl
{
    fn spawn<F>(&self, task: F)
    where
        F: std::future::Future<Output = ()>,
        F: Send + 'static,
    {
        std::thread::spawn(move || futures::executor::block_on(async move { task.await }));
    }
}

impl From<DefaultCPURuntime> for StdRuntimeImpl {
    fn from(_: DefaultCPURuntime) -> Self {
        StdRuntimeImpl{}
    }
}

impl From<DefaultCPURuntime> for &StdRuntimeImpl {
    fn from(_: DefaultCPURuntime) -> Self {
        &StdRuntimeImpl{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
