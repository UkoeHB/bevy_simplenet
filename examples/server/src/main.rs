//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::app::*;
use bevy::prelude::*;
use bevy_cobweb::prelude::*;

//standard shortcuts
use std::collections::HashSet;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

type DemoServer      = bevy_simplenet::Server<DemoChannel>;
type DemoServerEvent = bevy_simplenet::ServerEventFrom<DemoChannel>;

fn server_factory() -> bevy_simplenet::ServerFactory<DemoChannel>
{
    bevy_simplenet::ServerFactory::<DemoChannel>::new("demo")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct ClientConnections(HashSet<u128>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactResource, Default)]
struct ButtonState(Option<u128>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn setup(mut c: Commands)
{
    let _ = c.react().on(resource_mutation::<ButtonState>(), send_new_button_state);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_new_button_state(
    server  : Res<DemoServer>,
    clients : Res<ClientConnections>,
    state   : ReactRes<ButtonState>,
){
    for client_id in clients.0.iter()
    {
        server.send(*client_id, DemoServerMsg::Current(state.0));
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_server_events(
    mut c       : Commands,
    mut server  : ResMut<DemoServer>,
    mut clients : ResMut<ClientConnections>,
    mut state   : ReactResMut<ButtonState>,
){
    let mut new_button_state = state.0;

    while let Some((client_id, server_event)) = server.next()
    {
        match server_event
        {
            DemoServerEvent::Report(connection_report) => match connection_report
            {
                bevy_simplenet::ServerReport::Connected(_, _) =>
                {
                    // add client
                    let _ = clients.0.insert(client_id);

                    // send current server state to client
                    // - we must use new_button_state to ensure the order of events is preserved
                    let current_state = new_button_state;
                    server.send(client_id, DemoServerMsg::Current(current_state));
                }
                bevy_simplenet::ServerReport::Disconnected =>
                {
                    // remove client
                    let _ = clients.0.remove(&client_id);

                    // clear the state if disconnected client held the button
                    if state.0 == Some(client_id) { new_button_state = None; }
                }
            }
            DemoServerEvent::Msg(()) => continue,
            DemoServerEvent::Request(token, request) => match request
            {
                DemoClientRequest::Select =>
                {
                    // acknowldge selection
                    server.ack(token);

                    // update button
                    new_button_state = Some(client_id);
                }
            }
        }
    }

    // update button state if it changed
    // - we do this at the end
    //   A) so reactors aren't scheduled excessively
    //   B) because reactors are deferred, so to get the right order of events we must do this last
    if new_button_state == state.0 { return; }
    *state.get_mut(&mut c) = ButtonState(new_button_state);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    // prepare tracing
    // /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // */

    // simplenet server
    // - we use a baked-in address so you can close and reopen the server to test clients being disconnected
    let server = server_factory().new_server(
            enfync::builtin::native::TokioHandle::default(),
            "127.0.0.1:48888",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig{
                heartbeat_interval: std::time::Duration::from_secs(6),  //slower than client to avoid redundant pings
                ..Default::default()
            },
        );

    // prep server
    let mut app = App::empty();
    app
        .add_event::<AppExit>()
        .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(100)))
        .add_plugins(ReactPlugin)
        .init_schedule(Main)
        .insert_resource(server)
        .init_resource::<ClientConnections>()
        .insert_react_resource(ButtonState::default());

    // setup
    syscall(&mut app.world, (), setup);

    // run server
    app.add_systems(Main, handle_server_events)
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
