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
    pub(super) type DefaultIOReceiver<R>  = SimpleResultReceiver<TokioRuntimeImpl<R>, R>;
    pub(super) type DefaultCPUReceiver<R> = OneshotResultReceiver<StdRuntimeImpl, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(wasm)]
mod envmod
{
    use crate::*;
    pub(super) type DefaultIOReceiver<R>  = OneshotResultReceiver<WasmIORuntimeImpl, R>;
    /// note: We use the WASM IO runtime here because implementing a CPU runtime on WASM currently can only be done
    ///       with web workers, which are very inefficient and impose many restrictions on the interface (such as
    ///       requiring everything to implement `Serialize/Deserialize`, and needing explicitly-defined channels since
    ///       there is no shared memory).
    pub(super) type DefaultCPUReceiver<R> = OneshotResultReceiver<WasmIORuntimeImpl, R>;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

pub type DefaultIOPendingResult<R>  = PendingResult<envmod::DefaultIOReceiver<R>>;
pub type DefaultCPUPendingResult<R> = PendingResult<envmod::DefaultCPUReceiver<R>>;

//-------------------------------------------------------------------------------------------------------------------
