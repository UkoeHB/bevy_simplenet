//local shortcuts
use bevy_simplenet::*;

//third-party shortcuts

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[test]
fn pending_result_native_io()
{

}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn pending_result_native_cpu()
{
    // make cpu-oriented task
    dbg!("task");
    let task = async { dbg!("task ran"); };

    // spawn task
    let mut pending_result = DefaultCPUPendingResult::<()>::new((), task);

    // wait for task
    let PRResult::Result(_) = pending_result.extract() else { panic!(""); };
}

//-------------------------------------------------------------------------------------------------------------------
