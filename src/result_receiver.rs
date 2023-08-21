//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[async_trait::async_trait]
trait ResultReceiver
{
    pub type Runtime;
    pub type Result: Send + 'static;

    /// Make a new result receiver.
    pub fn new<T, F>(task: T, runtime: &Self::Runtime) -> Self
    where
        T: FnOnce() -> F + Send + 'static,
        F: std::future::Future<Output = Self::Result> + Send + 'static;

    /// Make a result receiver with an immediately-available result.
    pub fn immediate(result: Self::Result, runtime: &Self::Runtime) -> Self;

    /// Check if the result is ready.
    pub fn done(&self) -> bool;

    /// Get the result.
    /// Return `None` if the result could not be extracted (e.g. due to an error).
    pub async fn get(mut self) -> Option<Result>;
}

//-------------------------------------------------------------------------------------------------------------------

pub struct OneshotResultReceiver<Rt, R>
{
    oneshot: futures::channel::oneshot::Receiver<Option<R>>,
}

impl<Rt, R> ResultReceiver for OneshotResultReceiver<Rt, R>
where
    Rt: Into<OneshotRuntime>,
    R: Send + 'static
{
    type Runtime = Rt;
    type Result = R;

    pub fn new<T, F>(task: T, runtime: &Self::Runtime) -> Self
    where
        T: FnOnce() -> F + Send + 'static,
        F: std::future::Future<Output = Self::Result> + Send + 'static,
    {
        let (result_sender, result_receiver) = futures::channel::oneshot::channel();
        let work_task = async move {
                let Ok(result) = task().await else { let _ = result_sender.send(None); return; };
                let _ = result_sender.send(Some(result));
            };
        runtime.into::<OneshotRuntime>().spawn(work_task);

        Self{ oneshot: result_receiver }
    }

    pub fn immediate(result: Self::Result, _runtime: &Self::Runtime) -> Self
    {
        let (result_sender, result_receiver) = futures::channel::oneshot::channel();
        let _ = result_sender.send(Some(result));

        Self{ oneshot: result_receiver }
    }

    pub fn done(&self) -> bool
    {
        self.oneshot.is_terminated()
    }

    pub async fn get(mut self) -> Option<Result>
    {
        self.oneshot.await
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub struct SimpleResultReceiver<Rt: Into<SimpleRuntime<R>>, R>
{
    handle: <Rt as SimpleRuntime<R>>::Future<R>,
}

impl<Rt, R> ResultReceiver for SimpleResultReceiver<Rt, R>
where
    Rt: Into<SimpleRuntime<R>>,
    R: Send + 'static,
{
    type Runtime: Rt;
    type Result: R;

    pub fn new<T, F>(task: T, runtime: &Self::Runtime) -> Self
    where
        T: FnOnce() -> F + Send + 'static,
        F: std::future::Future<Output = Self::Result> + Send + 'static,
    {
        let handle = runtime.into::<SimpleRuntime<R>>().spawn(work_task);

        Self{ handle }
    }

    pub fn immediate(result: Self::Result, runtime: &Self::Runtime) -> Self
    {
        let handle = runtime.into::<SimpleRuntime<R>>().spawn(futures::future::ready::(result));

        Self{ handle }
    }

    pub fn done(&self) -> bool
    {
        self.handle.is_finished()
    }

    pub async fn get(mut self) -> Option<Result>
    {
        let Ok(result) = self.handle.await else { return None; };
        Some(result)
    }
}

//-------------------------------------------------------------------------------------------------------------------
