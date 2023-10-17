//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::app::*;
use bevy::prelude::*;
use bevy_kot::ecs::*;

//standard shortcuts
use std::collections::HashSet;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

type DemoServer = bevy_simplenet::Server<DemoChannel>;
type DemoClientVal = bevy_simplenet::ClientValFrom<DemoChannel>;

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

#[derive(Resource, Default)]
struct ButtonState(Option<u128>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn setup(mut react_commands: ReactCommands)
{
    let _ = react_commands.add_resource_mutation_reactor::<ButtonState>(
            move |world| { syscall(world, (), send_new_button_state); }
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_new_button_state(
    server  : Res<DemoServer>,
    clients : Res<ClientConnections>,
    state   : Res<ReactRes<ButtonState>>,
){
    for client_id in clients.0.iter()
    {
        let _ = server.send(*client_id, DemoServerMsg::Current(state.0));
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn update_client_connections(
    mut rcommands : ReactCommands,
    server        : Res<DemoServer>,
    mut clients   : ResMut<ClientConnections>,
    mut state     : ResMut<ReactRes<ButtonState>>
){
    while let Some(connection_report) = server.next_report()
    {
        match connection_report
        {
            bevy_simplenet::ServerReport::Connected(id, _, _) => { let _ = clients.0.insert(id); },
            bevy_simplenet::ServerReport::Disconnected(id) =>
            {
                let _ = clients.0.remove(&id);

                // clear the state if disconnected client held the button
                if state.0 == Some(id) { *state.get_mut(&mut rcommands) = ButtonState::default(); }
            },
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_client_incoming(
    mut rcommands : ReactCommands,
    server        : Res<DemoServer>,
    mut state     : ResMut<ReactRes<ButtonState>>
){
    let mut new_button_state = state.0;

    while let Some((client_id, client_val)) = server.next_val()
    {
        match client_val
        {
            DemoClientVal::Msg(()) => continue,
            DemoClientVal::Request(request, token) =>
            {
                match request
                {
                    DemoClientRequest::Select =>
                    {
                        // acknowldge selection
                        let _ = server.acknowledge(token);

                        // update button
                        new_button_state = Some(client_id);
                    }
                    DemoClientRequest::GetState =>
                    {
                        // send current server state to client
                        // - we must use new_button_state to ensure the order of events is preserved
                        let current_state = new_button_state;
                        let _ = server.respond(token, DemoServerResponse::Current(current_state));
                    }
                }
            }
        }
    }

    // update button state if it changed
    // - we do this at the end
    //   A) so reactors aren't scheduled excessively
    //   B) because reactors are deferred, so to get the right order of events we must do this last
    if new_button_state == state.0 { return; }
    *state.get_mut(&mut rcommands) = ButtonState(new_button_state);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    // prepare tracing
    // /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
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
        .insert_resource(ReactRes::new(ButtonState::default()));

    // setup
    syscall(&mut app.world, (), setup);

    // run server
    app.add_systems(Main,
            (
                update_client_connections, apply_deferred,
                handle_client_incoming, apply_deferred,
            ).chain()
        )
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
