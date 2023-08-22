//local shortcuts
use bevy_simplenet::*;

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[test]
fn pending_result_native_io()
{
    // make cpu-oriented task
    dbg!("task");
    let task = async { dbg!("task ran"); };

    // spawn task
    let mut pending_result = DefaultIOPendingResult::<()>::new(&DefaultIOHandle::default().into(), task);

    // wait for task
    let PRResult::Result(_) = pending_result.extract() else { panic!(""); };
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn pending_result_native_cpu()
{
    // make cpu-oriented task
    dbg!("task");
    let task = async { dbg!("task ran"); };

    // spawn task
    let mut pending_result = DefaultCPUPendingResult::<()>::new(DefaultCPUHandle::default(), task);

    // wait for task
    let PRResult::Result(_) = pending_result.extract() else { panic!(""); };
}

//-------------------------------------------------------------------------------------------------------------------
