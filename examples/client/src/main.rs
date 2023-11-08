//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_kot::prelude::{*, builtin::*};
use bevy_lunex::prelude::*;

//standard shortcuts
use std::fmt::Write;

#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_family = "wasm")]
use wasm_timer::{SystemTime, UNIX_EPOCH};

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

type DemoClient    = bevy_simplenet::Client<DemoChannel>;
type DemoServerVal = bevy_simplenet::ServerValFrom<DemoChannel>;

fn client_factory() -> bevy_simplenet::ClientFactory<DemoChannel>
{
    bevy_simplenet::ClientFactory::<DemoChannel>::new("demo")
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Copy, Clone, Eq, PartialEq, Debug)]
enum ConnectionStatus
{
    Connecting,
    Connected,
    Dead,
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn status_to_string(status: ConnectionStatus) -> &'static str
{
    match status
    {
        ConnectionStatus::Connecting => "connecting...",
        ConnectionStatus::Connected  => "connected",
        ConnectionStatus::Dead       => "DEAD",
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Component, Default)]
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
//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
struct ConnectionStatusFlag;

#[derive(Component)]
struct ButtonOwnerTextFlag;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
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
//-------------------------------------------------------------------------------------------------------------------

fn refresh_status_text(
    mut status_text : Query<&mut Text, With<ConnectionStatusFlag>>,
    status          : Res<ConnectionStatus>,
){
    if !status.is_changed() { return; }
    let text_section = &mut status_text.single_mut().sections[0].value;
    text_section.clear();
    let _ = write!(text_section, "Status: {}", status_to_string(*status));
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn refresh_button_owner_text(
    mut status_text : Query<&mut Text, With<ButtonOwnerTextFlag>>,
    current_state   : Query<&ButtonOwner, Changed<ButtonOwner>>,
){
    if current_state.is_empty() { return; }
    let text_section = &mut status_text.single_mut().sections[0].value;
    text_section.clear();
    match current_state.single().display_id()
    {
        Some(id) => { let _ = write!(text_section, "Owner: {}", id % 1_000_000u128); }
        None     => { let _ = write!(text_section, "No owner"); }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Handler for when the button is selected.
fn handle_button_select(
    mut commands      : Commands,
    client            : Res<DemoClient>,
    status            : Res<ConnectionStatus>,
    mut current_state : Query<(&mut PendingSelect, &mut ButtonOwner, &Callback<Deselect>)>,
){
    let (mut pending_select, mut owner, deselect_callback) = current_state.single_mut();

    // if not connected then we force-deselect
    if *status != ConnectionStatus::Connected
    {
        commands.add(deselect_callback.clone());
        return;
    }

    // send select request
    let Ok(signal) = client.request(DemoClientRequest::Select)
    else
    {
        commands.add(deselect_callback.clone());
        return;
    };

    // save the predicted input
    pending_select.0   = Some(signal);
    owner.predicted_id = Some(client.id());
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Handler for when the button is deselected.
fn handle_button_deselect(mut current_state : Query<(&mut PendingSelect, &mut ButtonOwner)>)
{
    let (mut pending_select, mut owner) = current_state.single_mut();

    // clear the input prediction
    pending_select.0   = None;
    owner.predicted_id = None;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn set_new_server_state(
    In(server_state)  : In<Option<u128>>,
    mut commands      : Commands,
    client            : Res<DemoClient>,
    mut current_state : Query<(&PendingSelect, &mut ButtonOwner, &Callback<Deselect>)>,
){
    let (pending_select, mut owner, deselect_callback) = current_state.single_mut();

    // update server state
    owner.server_authoritative_id = server_state;

    // check if we are predicted
    if pending_select.is_predicted() { return; }

    // if not predicted and server state doesn't match our id, deselect
    if server_state != Some(client.id())
    {
        commands.add(deselect_callback.clone());
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn connection_status_section(ui: &mut UiBuilder<MainUI>, area: Widget)
{
    // text layout helper
    let layout_helper = Widget::create(
            ui.tree(),
            area.end(""),
            RelativeLayout{  //add slight buffer around edge; extend y-axis to avoid resizing issues
                absolute_1: Vec2 { x: 5., y: 5. },
                absolute_2: Vec2 { x: -5., y: 0. },
                relative_1: Vec2 { x: 0., y: 0. },
                relative_2: Vec2 { x: 100., y: 200. },
                ..Default::default()
            }
        ).unwrap();

    // text widget
    let text = Widget::create(
            ui.tree(),
            layout_helper.end(""),
            SolidLayout::new()  //keep text in top right corner when window is resized
                .with_horizontal_anchor(1.0)
                .with_vertical_anchor(-1.0),
        ).unwrap();

    let text_style = TextStyle {
            font      : ui.asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size : 45.0,
            color     : Color::WHITE,
        };

    ui.commands().spawn(
            (
                TextElementBundle::new(
                    text,
                    TextParams::topleft().with_style(&text_style),
                    "Status: connecting..."  //use initial value to get correct initial text boundary
                ),
                ConnectionStatusFlag
            )
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn button_owner_section(ui: &mut UiBuilder<MainUI>, area: Widget)
{
    // text layout helper (extend y-axis to avoid resizing issues)
    let layout_helper = relative_widget(ui.tree(), area.end(""), (0., 100.), (0., 200.));

    // text widget
    let text = Widget::create(ui.tree(), layout_helper.end(""), SolidLayout::new()).unwrap();
    let text_style = TextStyle {
            font      : ui.asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size : 45.0,
            color     : Color::WHITE,
        };

    ui.commands().spawn(
            (
                TextElementBundle::new(
                    text,
                    TextParams::center().with_style(&text_style),
                    "Owner: 000000"  //use initial value to get correct initial text boundary
                ),
                ButtonOwnerTextFlag
            )
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn button_section(ui: &mut UiBuilder<MainUI>, area: Widget)
{
    // default button image tied to button
    let default_widget = make_overlay(ui.tree(), &area, "default", true);
    let image = ImageElementBundle::new(
            &default_widget,
            ImageParams::center()
                .with_width(Some(100.))
                .with_height(Some(100.))
                .with_color(Color::GRAY),
            ui.asset_server.load("example_button_rect.png"),
            Vec2::new(250.0, 142.0)
        );
    ui.commands().spawn(image);

    // selected button image tied to button
    let selected_widget = make_overlay(ui.tree(), &area, "selected", false);
    let image = ImageElementBundle::new(
            &selected_widget,
            ImageParams::center()
                .with_width(Some(100.))
                .with_height(Some(100.))
                .with_color(Color::DARK_GRAY),  //tint when selected
            ui.asset_server.load("example_button_rect.png"),
            Vec2::new(250.0, 142.0)
        );
    ui.commands().spawn(image);

    // button interactivity
    let mut entity_commands = ui.commands().spawn_empty();
    InteractiveElementBuilder::new()
        .with_default_widget(default_widget)
        .with_selected_widget(selected_widget)
        .select_on_click()
        .select_callback(|world| syscall(world, (), handle_button_select))
        .deselect_callback(|world| syscall(world, (), handle_button_deselect))
        .build::<MouseLButtonMain>(&mut entity_commands, area)
        .unwrap();
    entity_commands.insert(UIInteractionBarrier::<MainUI>::default());

    // cached select signal and server state tracking
    entity_commands.insert(
            (
                PendingSelect::default(),
                ButtonOwner::default(),
            )
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn build_ui(mut ui: UiBuilder<MainUI>)
{
    // root widget
    let root = relative_widget(ui.tree(), "root", (0., 100.), (0., 100.));

    // connection status text
    let text_base = relative_widget(ui.tree(), root.end("text"), (70., 100.), (0., 20.));
    connection_status_section(&mut ui, text_base);

    // button owner text
    let owner_base = relative_widget(ui.tree(), root.end("owner"), (37., 63.), (15., 35.));
    button_owner_section(&mut ui, owner_base);

    // button
    let button_base = relative_widget(ui.tree(), root.end("button"), (35., 65.), (40., 60.));
    button_section(&mut ui, button_base);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn setup(mut commands: Commands)
{
    // prepare 2D camera
    commands.spawn(
            Camera2dBundle{ transform: Transform{ translation: Vec3 { x: 0., y: 0., z: 1000. }, ..default() }, ..default() }
        );

    // make lunex cursor
    commands.spawn((Cursor::new(0.0), Transform::default(), MainMouseCursor));

    // prepare lunex ui tree
    commands.insert_resource(StyleStackRes::<MainUI>::default());
    commands.spawn((UiTree::new("ui"), MainUI));
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_connection_changes(
    client     : Res<DemoClient>,
    mut status : ResMut<ConnectionStatus>
){
    while let Some(connection_report) = client.next_report()
    {
        match connection_report
        {
            bevy_simplenet::ClientReport::Connected         =>
            {
                *status = ConnectionStatus::Connected;
                let _ = client.request(DemoClientRequest::GetState);
            },
            bevy_simplenet::ClientReport::Disconnected      |
            bevy_simplenet::ClientReport::ClosedByServer(_) |
            bevy_simplenet::ClientReport::ClosedBySelf      => *status = ConnectionStatus::Connecting,
            bevy_simplenet::ClientReport::IsDead            => *status = ConnectionStatus::Dead,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_server_incoming(
    mut commands      : Commands,
    client            : Res<DemoClient>,
    mut current_state : Query<(&mut PendingSelect, &mut ButtonOwner, &Callback<Deselect>)>,
){
    let (mut pending_select, mut owner, deselect_callback) = current_state.single_mut();

    while let Some(server_val) = client.next_val()
    {
        match server_val
        {
            DemoServerVal::Msg(message) =>
            {
                match message
                {
                    DemoServerMsg::Current(new_id) =>
                    {
                        commands.add(move |world: &mut World| syscall(world, new_id, set_new_server_state));
                    }
                }
            }
            DemoServerVal::Response(response, _request_id) =>
            {
                match response
                {
                    DemoServerResponse::Current(new_id) =>
                    {
                        commands.add(move |world: &mut World| syscall(world, new_id, set_new_server_state));
                    }
                }
            }
            DemoServerVal::Ack(request_id) =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // merge predicted input
                owner.server_authoritative_id = owner.predicted_id;
                owner.predicted_id            = None;
                pending_select.0              = None;
            }
            DemoServerVal::Reject(request_id) =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // roll back predicted input
                commands.add(deselect_callback.clone());
            }
            DemoServerVal::SendFailed(request_id)   |
            DemoServerVal::ResponseLost(request_id) |
            DemoServerVal::Aborted(request_id) =>
            {
                if !pending_select.equals_request(request_id) { continue; }

                // an error occurred, roll back the predicted input
                commands.add(deselect_callback.clone());
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
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
            focused_mode   : UpdateMode::Reactive{ max_wait: std::time::Duration::from_millis(100) },
            unfocused_mode : UpdateMode::Reactive{ max_wait: std::time::Duration::from_millis(100) },
            ..Default::default()
        })
        .add_plugins(LunexUiPlugin)
        .register_interaction_source(MouseLButtonMain::default())
        .insert_resource(client)
        .insert_resource(ConnectionStatus::Connecting)
        .add_systems(PreStartup, setup)
        .add_systems(Startup, build_ui)
        .add_systems(PreUpdate, handle_connection_changes)
        .add_systems(Update,
            (
                handle_server_incoming, apply_deferred,
                refresh_status_text,
                refresh_button_owner_text,
            ).chain()
        )
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
