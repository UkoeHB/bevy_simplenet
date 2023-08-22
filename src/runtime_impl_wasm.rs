//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Implements `OneshotRuntime` for `wasm` runtimes (spawn on local thread).
/// If no other type implements `From<DefaultIOHandle>`, this is the default IO runtime on WASM builds.
/// If no other type implements `From<DefaultCPUHandle>`, this is the default CPU runtime on WASM builds.
#[derive(Debug, Clone, Default)]
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

impl From<DefaultIOHandle> for WasmIORuntime {
    fn from(_: DefaultIOHandle) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultIOHandle> for &WasmIORuntime {
    fn from(_: DefaultIOHandle) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultCPUHandle> for WasmIORuntime {
    fn from(_: DefaultCPUHandle) -> Self {
        WasmIORuntime{}
    }
}

impl From<DefaultCPUHandle> for &WasmIORuntime {
    fn from(_: DefaultCPUHandle) -> Self {
        WasmIORuntime{}
    }
}

//-------------------------------------------------------------------------------------------------------------------
