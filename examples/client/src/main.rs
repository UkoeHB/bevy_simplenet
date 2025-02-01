//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;

//standard shortcuts
use std::fmt::Write;
use wasm_timer::{SystemTime, UNIX_EPOCH};

//-------------------------------------------------------------------------------------------------------------------

type DemoClient      = bevy_simplenet::Client<DemoChannel>;
type DemoClientEvent = bevy_simplenet::ClientEventFrom<DemoChannel>;

fn client_factory() -> bevy_simplenet::ClientFactory<DemoChannel>
{
    bevy_simplenet::ClientFactory::<DemoChannel>::new("demo")
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactResource, Copy, Clone, Eq, PartialEq, Debug)]
enum ConnectionStatus
{
    Connecting,
    Connected,
    Dead,
}

impl ConnectionStatus
{
    fn to_string(&self) -> &'static str
    {
        match *self
        {
            ConnectionStatus::Connecting => "connecting...",
            ConnectionStatus::Connected  => "connected",
            ConnectionStatus::Dead       => "DEAD",
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactResource, Default)]
struct ButtonOwner
{
    server_authoritative_id : Option<u128>,
    predicted_id            : Option<u128>
}

impl ButtonOwner
{
    fn display_id(&self) -> Option<u128>
    {
        if self.predicted_id.is_some() { return self.predicted_id }
        self.server_authoritative_id
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactResource)]
struct PendingSelect(Option<bevy_simplenet::RequestSignal>);

impl PendingSelect
{
    fn equals_request(&self, request_id: u64) -> bool
    {
        let Some(signal) = &self.0 else { return false; };
        signal.id() == request_id
    }

    fn is_predicted(&self) -> bool
    {
        self.0.is_some()
    }
}

impl Default for PendingSelect { fn default() -> Self { Self(None) } }

//-------------------------------------------------------------------------------------------------------------------

/// Event broadcasted for when the button should be selected.
struct SelectButton;

/// Event broadcasted for when the button should be deselected.
struct DeselectButton;

//-------------------------------------------------------------------------------------------------------------------

fn handle_button_select(
    mut c: Commands,
    client: Res<DemoClient>,
    status: ReactRes<ConnectionStatus>,
    mut pending_select: ReactResMut<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
)
{
    // if not connected then we force-deselect
    if *status != ConnectionStatus::Connected
    {
        c.react().broadcast(DeselectButton);
        return;
    }

    // send select request
    let signal = client.request(DemoClientRequest::Select);

    // save the predicted input
    pending_select.get_mut(&mut c).0   = Some(signal);
    owner.get_mut(&mut c).predicted_id = Some(client.id());
}

//-------------------------------------------------------------------------------------------------------------------

fn handle_button_deselect(
    mut c: Commands,
    mut pending_select: ReactResMut<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
)
{
    pending_select.get_mut(&mut c).0   = None;
    owner.get_mut(&mut c).predicted_id = None;
}

//-------------------------------------------------------------------------------------------------------------------

fn set_new_server_state(
    In(server_state) : In<Option<u128>>,
    mut c            : Commands,
    client           : Res<DemoClient>,
    pending_select   : ReactRes<PendingSelect>,
    mut owner        : ReactResMut<ButtonOwner>
){
    // update server state
    owner.get_mut(&mut c).server_authoritative_id = server_state;

    // check if we are predicted
    if pending_select.is_predicted() { return; }

    // if not predicted and server state doesn't match our id, deselect
    if server_state != Some(client.id())
    {
        c.react().broadcast(DeselectButton);
    }
}

//-------------------------------------------------------------------------------------------------------------------

fn build_ui(mut c: Commands, mut s: SceneBuilder)
{
    c.ui_root().spawn_scene(("example.client", "scene"), &mut s, |l| {
        l.edit("status", |l| {
            l.update_on(resource_mutation::<ConnectionStatus>(),
                |id: TargetId, mut e: TextEditor, status: ReactRes<ConnectionStatus>| {
                    write_text!(e, *id, "Status: {}", status.to_string());
                }
            );
        })
        .edit("owner", |l| {
            l.update_on(resource_mutation::<ButtonOwner>(),
                |id: TargetId, mut e: TextEditor, owner: ReactRes<ButtonOwner>| {
                    let _ = match owner.display_id()
                    {
                        Some(display_id) => write_text!(e, *id, "Owner: {}", display_id % 1_000_000u128),
                        None => write_text!(e, *id, "No owner"),
                    };
                }
            );
        });

        l.spawn_scene(("example.client", "button"), |l| {
            let button = l.id();
            l.on_pressed(move |mut c: Commands| {
                c.react().entity_event(button, Select);
                c.react().broadcast(SelectButton);
            })
            .update_on(broadcast::<DeselectButton>(), |id: TargetId, mut c: Commands| {
                c.react().entity_event(*id, Deselect);
            })
            .on_select(|| println!("selected"))
            .on_deselect(|| println!("deselected"));
        });
    });
}

//-------------------------------------------------------------------------------------------------------------------

fn setup(mut commands: Commands)
{
    // prepare 2D camera
    commands.spawn(Camera2d::default());
}

//-------------------------------------------------------------------------------------------------------------------

fn handle_client_events(
    mut c              : Commands,
    mut client         : ResMut<DemoClient>,
    mut status         : ReactResMut<ConnectionStatus>,
    mut pending_select : ReactResMut<PendingSelect>,
    mut owner          : ReactResMut<ButtonOwner>
){
    let mut next_status = *status;

    while let Some(client_event) = client.next()
    {
        match client_event
        {
            DemoClientEvent::Report(connection_report) => match connection_report
            {
                bevy_simplenet::ClientReport::Connected         => next_status = ConnectionStatus::Connected,
                bevy_simplenet::ClientReport::Disconnected      |
                bevy_simplenet::ClientReport::ClosedByServer(_) |
                bevy_simplenet::ClientReport::ClosedBySelf      => next_status = ConnectionStatus::Connecting,
                bevy_simplenet::ClientReport::IsDead(aborted_reqs) =>
                {
                    for aborted_req in aborted_reqs
                    {
                        if !pending_select.equals_request(aborted_req) { continue; }

                        // an error occurred, roll back the predicted input
                        c.react().broadcast(DeselectButton);
                    }
                    next_status = ConnectionStatus::Dead;
                }
            }
            DemoClientEvent::Msg(message) => match message
            {
                DemoServerMsg::Current(new_id) =>
                {
                    // reset current state
                    c.syscall(new_id, set_new_server_state);
                }
            }
            DemoClientEvent::Ack(request_id) =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // merge predicted input
                let owner = owner.get_mut(&mut c);
                owner.server_authoritative_id = owner.predicted_id;
                owner.predicted_id = None;
                pending_select.get_mut(&mut c).0 = None;
            }
            DemoClientEvent::Reject(request_id) =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // roll back predicted input
                c.react().broadcast(DeselectButton);
            }
            DemoClientEvent::Response((), request_id) |
            DemoClientEvent::SendFailed(request_id)    |
            DemoClientEvent::ResponseLost(request_id)  =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // an error occurred, roll back the predicted input
                c.react().broadcast(DeselectButton);
            }
        }
    }

    if next_status != *status {
        *status.get_mut(&mut c) = next_status;
    }
}

//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    // setup wasm tracing
    #[cfg(target_family = "wasm")]
    {
        //console_error_panic_hook::set_once();
        //tracing_wasm::set_as_global_default();
    }

    // simplenet client
    // - we use a baked-in address so you can close and reopen the server to test clients being disconnected
    let client = client_factory().new_client(
            enfync::builtin::Handle::default(),  //automatically selects native/WASM runtime
            url::Url::parse("ws://127.0.0.1:48888/ws").unwrap(),
            bevy_simplenet::AuthRequest::None{
                client_id: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
            },
            bevy_simplenet::ClientConfig{
                reconnect_on_disconnect   : true,
                reconnect_on_server_close : true,
                ..Default::default()
            },
            ()
        );

    // prepare bevy plugins
    let bevy_plugins = bevy::DefaultPlugins
        .set(
            WindowPlugin{
                primary_window: Some(Window{ window_theme: Some(WindowTheme::Dark), ..Default::default() }),
                ..Default::default()
            }
        );

    // reduce input lag on native targets
    #[cfg(not(target_family = "wasm"))]
    let bevy_plugins = bevy_plugins.build().disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>();

    // run client
    App::new()
        .add_plugins(bevy_plugins)
        .insert_resource(WinitSettings{
            focused_mode   : UpdateMode::reactive(std::time::Duration::from_millis(100)),
            unfocused_mode : UpdateMode::reactive(std::time::Duration::from_millis(100)),
            ..Default::default()
        })
        .add_plugins(ReactPlugin)
        .add_plugins(CobwebUiPlugin)
        .load("main.cob")
        .insert_resource(client)
        .insert_react_resource(ConnectionStatus::Connecting)
        .init_react_resource::<ButtonOwner>()
        .init_react_resource::<PendingSelect>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(LoadState::Done), build_ui)
        .add_systems(Update, handle_client_events)
        .add_reactor(broadcast::<SelectButton>(), handle_button_select)
        .add_reactor(broadcast::<DeselectButton>(), handle_button_deselect)
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
