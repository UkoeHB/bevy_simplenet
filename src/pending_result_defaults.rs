//local shortcuts
use crate::*;

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(not(wasm))]
mod envmod
{
    use crate::*;
    pub(super) type DefaultIOReceiver<R>  = SimpleResultReceiver<TokioRuntime<R>, R>;
    pub(super) type DefaultCPUReceiver<R> = OneshotResultReceiver<StdRuntime, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(wasm)]
mod envmod
{
    use crate::*;
    type DefaultIOReceiver<R>  = OneshotResultReceiver<WasmIORuntime, R>;
    type DefaultCPUReceiver<R> = OneshotResultReceiver<WasmIORuntime, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

pub type DefaultIOPendingResult<R>  = PendingResult<envmod::DefaultIOReceiver<R>>;
pub type DefaultCPUPendingResult<R> = PendingResult<envmod::DefaultCPUReceiver<R>>;

//-------------------------------------------------------------------------------------------------------------------
