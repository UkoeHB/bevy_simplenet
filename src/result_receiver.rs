//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts
use futures::future::FusedFuture;


//-------------------------------------------------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait ResultReceiver
{
    type Runtime;
    type Result: Send + 'static;

    /// Make a new result receiver.
    fn new<F>(runtime: &Self::Runtime, task: F) -> Self
    where
        F: std::future::Future<Output = Self::Result> + Send + 'static;

    /// Make a result receiver with an immediately-available result.
    fn immediate(runtime: &Self::Runtime, result: Self::Result) -> Self;

    /// Check if the result is ready.
    fn done(&self) -> bool;

    /// Get the result.
    /// Return `None` if the result could not be extracted (e.g. due to an error).
    async fn get(mut self) -> Option<Self::Result>;
}

//-------------------------------------------------------------------------------------------------------------------

pub struct OneshotResultReceiver<Rt, R>
{
    oneshot: futures::channel::oneshot::Receiver<Option<R>>,
    _phantom: std::marker::PhantomData<Rt>,
}

#[async_trait::async_trait]
impl<Rt, R> ResultReceiver for OneshotResultReceiver<Rt, R>
where
    Rt: OneshotRuntime,
    R: Send + 'static
{
    type Runtime = Rt;
    type Result = R;

    fn new<F>(runtime: &Self::Runtime, task: F) -> Self
    where
        F: std::future::Future<Output = Self::Result> + Send + 'static,
    {
        let (result_sender, result_receiver) = futures::channel::oneshot::channel();
        let work_task = async move {
                let result = task.await; //else { let _ = result_sender.send(None); return; };
                let _ = result_sender.send(Some(result));
            };
        runtime.spawn(work_task);

        Self{ oneshot: result_receiver, _phantom: std::marker::PhantomData::<Self::Runtime>::default() }
    }

    fn immediate(_runtime: &Self::Runtime, result: Self::Result) -> Self
    {
        let (result_sender, result_receiver) = futures::channel::oneshot::channel();
        let _ = result_sender.send(Some(result));

        Self{ oneshot: result_receiver, _phantom: std::marker::PhantomData::<Self::Runtime>::default() }
    }

    fn done(&self) -> bool
    {
        self.oneshot.is_terminated()
    }

    async fn get(mut self) -> Option<Self::Result>
    {
        self.oneshot.await.unwrap_or(None)
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub struct SimpleResultReceiver<Rt: SimpleRuntime<R>, R>
{
    handle: <Rt as SimpleRuntime<R>>::Future,
}

#[async_trait::async_trait]
impl<Rt, R> ResultReceiver for SimpleResultReceiver<Rt, R>
where
    Rt: SimpleRuntime<R>,
    R: Send + 'static,
{
    type Runtime = Rt;
    type Result = R;

    fn new<F>(runtime: &Self::Runtime, task: F) -> Self
    where
        F: std::future::Future<Output = Self::Result> + Send + 'static,
    {
        let handle = runtime.spawn(task);

        Self{ handle }
    }

    fn immediate(runtime: &Self::Runtime, result: Self::Result) -> Self
    {
        let handle = runtime.spawn(futures::future::ready(result));

        Self{ handle }
    }

    fn done(&self) -> bool
    {
        Self::Runtime::is_terminated(&self.handle)
    }

    async fn get(mut self) -> Option<Self::Result>
    {
        let Ok(result) = self.handle.await else { return None; };
        Some(result)
    }
}

//-------------------------------------------------------------------------------------------------------------------
