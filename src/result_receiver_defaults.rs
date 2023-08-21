//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(not(wasm))]
mod envmod
{
    type DefaultIOReceiver<R>  = SimpleResultReceiver<TokioRuntime<R>, R>;
    type DefaultCPUReceiver<R> = OneshotResultReceiver<StdRuntime, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(wasm)]
mod envmod
{
    type DefaultIOReceiver<R>  = OneshotResultReceiver<WasmIORuntime, R>;
    type DefaultCPUReceiver<R> = OneshotResultReceiver<WasmIORuntime, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

pub type DefaultIOReceiver<R>  = envmod::DefaultIOReceiver<R>;
pub type DefaultCPUReceiver<R> = envmod::DefaultCPUReceiver<R>;

//-------------------------------------------------------------------------------------------------------------------
