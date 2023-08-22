//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use std::fmt::Debug;

//-------------------------------------------------------------------------------------------------------------------

/// Implements `SimpleRuntime` for `tokio` runtimes (spawn on tokio runtime).
/// If no other type implements `From<DefaultIOHandle>`, this is the default IO runtime on native builds.
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
        Self::from(runtime.handle().clone())
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

impl<R: Send + 'static> From<DefaultIOHandle> for TokioRuntimeImpl<R> {
    fn from(handle: DefaultIOHandle) -> Self {
        Self::from(tokio::runtime::Handle::from(handle))
    }
}

impl<R: Send + 'static> From<&DefaultIOHandle> for TokioRuntimeImpl<R> {
    fn from(handle: &DefaultIOHandle) -> Self {
        Self::from(handle.clone())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Implements `OneshotRuntime` for `std` runtimes (spawn new thread).
/// If no other type implements `From<DefaultCPUHandle>`, this is the default CPU runtime on native builds.
#[derive(Debug, Clone, Default)]
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

impl From<DefaultCPUHandle> for StdRuntimeImpl {
    fn from(_: DefaultCPUHandle) -> Self {
        StdRuntimeImpl{}
    }
}

impl From<DefaultCPUHandle> for &StdRuntimeImpl {
    fn from(_: DefaultCPUHandle) -> Self {
        &StdRuntimeImpl{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
