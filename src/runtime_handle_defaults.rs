//local shortcuts

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(not(wasm))]
mod envmod
{
    use crate::*;

    /// Default IO runtime handle (tokio).
    /// If you access this via `::default()`, you will get a handle to a statically-initialized tokio runtime.
    #[derive(Clone, Debug)]
    pub struct DefaultIOHandle(pub tokio::runtime::Handle);

    impl Default for DefaultIOHandle
    {
        fn default() -> DefaultIOHandle
        {
            static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

            let runtime = RUNTIME.get_or_init(
                    || {
                        tokio::runtime::Runtime::new().expect("unable to get default tokio runtime")
                    }
                );
            DefaultIOHandle(runtime.handle().clone())
        }
    }

    impl TryAdopt for DefaultIOHandle
    {
        fn try_adopt() -> Option<DefaultIOHandle>
        {
            let Ok(handle) = tokio::runtime::Handle::try_current() else { return None; };
            Some(DefaultIOHandle::from(handle))
        }
    }

    impl From<DefaultIOHandle> for tokio::runtime::Handle {
        fn from(runtime: DefaultIOHandle) -> Self {
            runtime.0
        }
    }

    impl From<&DefaultIOHandle> for tokio::runtime::Handle {
        fn from(runtime: &DefaultIOHandle) -> Self {
            runtime.0.clone()
        }
    }

    impl From<tokio::runtime::Handle> for DefaultIOHandle {
        fn from(handle: tokio::runtime::Handle) -> Self {
            Self(handle)
        }
    }

    impl From<&tokio::runtime::Handle> for DefaultIOHandle {
        fn from(handle: &tokio::runtime::Handle) -> Self {
            Self(handle.clone())
        }
    }

    /// Default CPU runtime handle (unspecified)
    #[derive(Default)]
    pub struct DefaultCPUHandle;

    impl TryAdopt for DefaultCPUHandle
    {
        fn try_adopt() -> Option<DefaultCPUHandle>
        {
            Some(DefaultCPUHandle)
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[cfg(wasm)]
mod envmod
{
    use crate::*;

    /// Default IO runtime handle (unspecified)
    #[derive(Clone, Debug, Default)]
    pub struct DefaultIOHandle;

    impl TryAdopt for DefaultIOHandle
    {
        fn try_adopt() -> Option<DefaultIOHandle>
        {
            Some(DefaultIOHandle)
        }
    }

    /// Default CPU runtime handle (unspecified)
    #[derive(Clone, Debug, Default)]
    pub struct DefaultCPUHandle;

    impl TryAdopt for DefaultCPUHandle
    {
        fn try_adopt() -> Option<DefaultCPUHandle>
        {
            Some(DefaultCPUHandle)
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Try to adopt the existing runtime.
/// Returns `None` if no runtime is detected.
pub trait TryAdopt: Sized
{
    fn try_adopt() -> Option<Self>;
}

//-------------------------------------------------------------------------------------------------------------------

/// Try to adopt the existing runtime, otherwise fall back to the default runtime.
pub trait AdoptOrDefault: TryAdopt + Default
{
    fn adopt_or_default() -> Self
    {
        if let Some(adoptee) = Self::try_adopt() { return adoptee; }
        Self::default()
    }
}

impl<T: TryAdopt + Default> AdoptOrDefault for T {}

//-------------------------------------------------------------------------------------------------------------------

pub type DefaultIOHandle  = envmod::DefaultIOHandle;
pub type DefaultCPUHandle = envmod::DefaultCPUHandle;

//-------------------------------------------------------------------------------------------------------------------
