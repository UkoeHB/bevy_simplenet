//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

pub struct WasmIORuntime;

impl OneshotRuntime for WasmIORuntime
{
    fn spawn<T, F>(&self, task: T)
    where
        T: FnOnce() -> F,
        T: Send + 'static,
        F: std::future::Future<Output = ()>,
        F: Send + 'static,
    {
        wasm_bindgen_futures::spawn_local(
                async move {
                        task().await;
                    }
            );
    }
}

impl From<()> for WasmIORuntime
{
    fn from(_: ()) -> Self
    {
        WasmIORuntime{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
