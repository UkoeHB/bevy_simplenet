//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

pub trait OneshotRuntime
{
    pub fn spawn<T, F>(&self, task: T)
    where
        T: FnOnce() -> F,
        F: std::future::Future<Output = ()>;
}

//-------------------------------------------------------------------------------------------------------------------

pub trait SimpleRuntime<R>
{
    pub type Result = R;
    pub type Future: std::future::Future<Output = Self::Result>;

    pub fn spawn<T, F>(&self, task: T) -> Self::Future
    where
        T: FnOnce() -> F,
        F: std::future::Future<Output = Self::Result>;
}

impl<SRt: SimpleRuntime::<()>> OneshotRuntime for SRt
{
    pub fn spawn<T, F>(&self, task: T)
    where
        T: FnOnce() -> F,
        F: std::future::Future<Output = ()>
    {
        self.<SRt as SimpleRuntime::<()>>::spawn(task);  //discard future (assume oneshots use a oneshot channel)
    }
}

//-------------------------------------------------------------------------------------------------------------------
