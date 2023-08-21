//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

pub struct TokioRuntime<R>
{
    handle: tokio::runtime::Handle,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Send + 'static> SimpleRuntime<R> for TokioRuntime<R>
{
    type Future = tokio::runtime::JoinHandle<Self::Result>;

    fn spawn<T, F>(&self, task: T) -> Self::Future
    where
        T: FnOnce() -> F,
        T: Send + 'static,
        F: std::future::Future<Output = Self::Result>,
        F: Send + 'static,
    {
        self.handle.spawn(task)
    }
}

impl<R: Send + 'static> From<&tokio::runtime::Runtime> for TokioRuntime<R>
{
    fn from(runtime: &tokio::runtime::Runtime) -> Self
    {
        Self{ handle: runtime.handle().clone(), _phantom: std::marker::PhantomData<R>::default() }
    }
}

impl<R: Send + 'static> From<tokio::runtime::Runtime> for TokioRuntime<R>
{
    fn from(runtime: tokio::runtime::Runtime) -> Self
    {
        Self::from(&runtime)
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub struct StdRuntime;

impl OneshotRuntime for StdRuntime
{
    fn spawn<T, F>(&self, task: T)
    where
        T: FnOnce() -> F,
        T: Send + 'static,
        F: std::future::Future<Output = ()>,
        F: Send + 'static,
    {
        std::thread::spawn(task);
    }
}

impl From<()> for StdRuntime
{
    fn from(_: ()) -> Self
    {
        StdRuntime{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
