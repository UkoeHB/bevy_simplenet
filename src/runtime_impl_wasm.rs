//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Implements `OneshotRuntime` for `wasm` runtimes (spawn on local thread).
/// If no other type implements `DefaultIORuntime`, this is the default IO runtime on WASM builds.
/// If no other type implements `DefaultCPURuntime`, this is the default CPU runtime on WASM builds.
#[derive(Debug)]
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

impl From<DefaultIORuntime> for WasmIORuntime {
    fn from(_: DefaultIORuntime) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultIORuntime> for &WasmIORuntime {
    fn from(_: DefaultIORuntime) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultCPURuntime> for WasmIORuntime {
    fn from(_: DefaultCPURuntime) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultCPURuntime> for &WasmIORuntime {
    fn from(_: DefaultCPURuntime) -> Self {
        WasmIORuntime{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
