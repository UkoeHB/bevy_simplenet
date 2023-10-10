//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::app::*;
use bevy::prelude::*;

//standard shortcuts
use std::collections::HashSet;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

type DemoServer = bevy_simplenet::Server::<DemoMsgPack>;
type DemoClientVal = bevy_simplenet::ClientValFromPack<DemoMsgPack>;

fn server_factory() -> bevy_simplenet::ServerFactory<DemoMsgPack>
{
    bevy_simplenet::ServerFactory::<DemoMsgPack>::new("demo")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct ClientConnections(HashSet<u128>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn update_client_connections(server: Res<DemoServer>, mut clients: ResMut<ClientConnections>)
{
    while let Some(connection_report) = server.next_report()
    {
        match connection_report
        {
            bevy_simplenet::ServerReport::Connected(id, _) => { let _ = clients.0.insert(id); },
            bevy_simplenet::ServerReport::Disconnected(id) => { let _ = clients.0.remove(&id); },
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_client_incoming(server: Res<DemoServer>, clients: Res<ClientConnections>)
{
    while let Some((client_id, DemoClientVal::Msg(message))) = server.next_val()
    {
        match message
        {
            DemoClientMsg::Select =>
            {
                // ack the select
                let _ = server.send(client_id, DemoServerMsg::AckSelect);

                // tell other clients to deselect
                for other_client in clients.0.iter()
                {
                    if *other_client == client_id { continue; };
                    let _ = server.send(*other_client, DemoServerMsg::Deselect);
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    // simplenet server
    // - we use a baked-in address so you can close and reopen the server to test clients being disconnected
    let server = server_factory().new_server(
            enfync::builtin::native::TokioHandle::default(),
            "127.0.0.1:48888",
            bevy_simplenet::AcceptorConfig::Default,
            bevy_simplenet::Authenticator::None,
            bevy_simplenet::ServerConfig::default(),
        );

    // run server
    App::empty()
        .add_event::<AppExit>()
        .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(100)))
        .init_schedule(Main)
        .insert_resource(server)
        .init_resource::<ClientConnections>()
        .add_systems(Main,
            (
                update_client_connections,
                handle_client_incoming
            ).chain()
        )
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
