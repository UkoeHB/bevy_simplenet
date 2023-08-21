//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(not(wasm))]
mod envmod
{
    use crate::*;
    #[derive(Clone)]
    pub struct DefaultIORuntime(pub tokio::runtime::Handle);

    impl Default for DefaultIORuntime
    {
        fn default() -> DefaultIORuntime
        {
            static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

            let runtime = RUNTIME.get_or_init(
                    || {
                        tokio::runtime::Runtime::new().expect("unable to get default tokio runtime")
                    }
                );
            DefaultIORuntime(runtime.handle().clone())
        }
    }

    impl From<DefaultIORuntime> for tokio::runtime::Handle
    {
        fn from(runtime: DefaultIORuntime) -> Self
        {
            runtime.0
        }
    }

    impl From<&DefaultIORuntime> for tokio::runtime::Handle
    {
        fn from(runtime: &DefaultIORuntime) -> Self
        {
            runtime.0.clone()
        }
    }

    impl From<tokio::runtime::Handle> for DefaultIORuntime
    {
        fn from(handle: tokio::runtime::Handle) -> Self
        {
            Self(handle)
        }
    }

    impl From<&tokio::runtime::Handle> for DefaultIORuntime
    {
        fn from(handle: &tokio::runtime::Handle) -> Self
        {
            Self(handle.clone())
        }
    }

    pub type DefaultCPURuntime = EmptyRuntime;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(wasm)]
mod envmod
{
    use crate::*;
    pub type DefaultIORuntime  = EmptyRuntime;
    pub type DefaultCPURuntime = EmptyRuntime;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct EmptyRuntime;

//-------------------------------------------------------------------------------------------------------------------

pub type DefaultIORuntime  = envmod::DefaultIORuntime;
pub type DefaultCPURuntime = envmod::DefaultCPURuntime;

//-------------------------------------------------------------------------------------------------------------------
