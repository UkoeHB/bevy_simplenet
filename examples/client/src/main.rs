//local shortcuts
use bevy_simplenet_common::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_kot::ecs::*;
use bevy_kot::ui::{*, RegisterInteractionSourceExt};
use bevy_kot::ui::builtin::*;
use bevy_lunex::prelude::*;

//standard shortcuts
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

type DemoClient = bevy_simplenet::Client<DemoChannel>;
type DemoServerVal = bevy_simplenet::ServerVal<DemoChannel>;

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

#[derive(Component)]
struct ConnectionStatusFlag;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
struct PendingSelect(Option<bevy_simplenet::MessageSignal>);

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

fn handle_button_select(
    mut commands      : Commands,
    client            : Res<DemoClient>,
    status            : Res<ConnectionStatus>,
    mut current_state : Query<(&mut PendingSelect, &Callback<Deselect>)>,
){
    let (mut pending_select, deselect_callback) = current_state.single_mut();

    // if not connected then we force-deselect
    if *status != ConnectionStatus::Connected
    {
        commands.add(deselect_callback.clone());
        return;
    }

    // send select message
    let Ok(signal) = client.send(DemoClientMsg::Select)
    else
    {
        commands.add(deselect_callback.clone());
        return;
    };

    // cache the signal
    pending_select.0 = Some(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn connection_status_section(commands: &mut Commands, asset_server: &AssetServer, ui: &mut UiTree, text_base: Widget)
{
    // text layout helper
    let layout_helper = Widget::create(
            ui,
            text_base.end(""),
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
            ui,
            layout_helper.end(""),
            SolidLayout::new()
                .with_horizontal_anchor(1.0)
                .with_vertical_anchor(-1.0),
        ).unwrap();

    let text_style = TextStyle {
            font      : asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size : 45.0,
            color     : Color::WHITE,
        };

    commands.spawn(
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

fn button_section(commands: &mut Commands, asset_server: &AssetServer, ui: &mut UiTree, button_base: Widget)
{
    // default button image tied to button
    let default_widget = make_overlay(ui, &button_base, "default", true);
    commands.spawn(
            ImageElementBundle::new(
                    &default_widget,
                    ImageParams::center()
                        .with_width(Some(100.))
                        .with_height(Some(100.))
                        .with_color(Color::GRAY),
                    asset_server.load("example_button_rect.png"),
                    Vec2::new(250.0, 142.0)
                )
        );

    // selected button image tied to button
    let selected_widget = make_overlay(ui, &button_base, "selected", false);
    commands.spawn(
            ImageElementBundle::new(
                    &selected_widget,
                    ImageParams::center()
                        .with_width(Some(100.))
                        .with_height(Some(100.))
                        .with_color(Color::DARK_GRAY),  //tint when selected
                    asset_server.load("example_button_rect.png"),
                    Vec2::new(250.0, 142.0)
                )
        );

    // button interactivity
    let mut entity_commands = commands.spawn_empty();
    InteractiveElementBuilder::new()
        .with_default_widget(default_widget)
        .with_selected_widget(selected_widget)
        .select_on_click()
        .select_callback(|world| syscall(world, (), handle_button_select))
        .build::<MouseLButtonMain>(&mut entity_commands, button_base)
        .unwrap();
    entity_commands.insert(UIInteractionBarrier::<MainUI>::default());

    // cached select signal
    entity_commands.insert(PendingSelect::default());
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn setup(mut commands: Commands, asset_server: Res<AssetServer>)
{
    // prepare 2D camera
    commands.spawn(
            Camera2dBundle{ transform: Transform{ translation: Vec3 { x: 0., y: 0., z: 1000. }, ..default() }, ..default() }
        );

    // make lunex cursor
    commands.spawn((Cursor::new(0.0), Transform::default(), MainMouseCursor));

    // create lunex ui tree
    let mut ui = UiTree::new("ui");

    // root widget
    let root = Widget::create(
            &mut ui,
            "root",
            RelativeLayout{
                relative_1 : Vec2 { x: 0.0, y: 0.0 },
                relative_2 : Vec2 { x: 100.0, y: 100.0 },
                ..Default::default()
            }
        ).unwrap();

    // connection status text
    let text_base = Widget::create(
            &mut ui,
            root.end("text"),
            RelativeLayout{  //upper right corner
                relative_1: Vec2 { x: 70., y: 0. },
                relative_2: Vec2 { x: 100., y: 20. },
                ..Default::default()
            }
        ).unwrap();
    connection_status_section(&mut commands, &asset_server, &mut ui, text_base);

    // button
    let button_base = Widget::create(
            &mut ui,
            root.end("button"),
            RelativeLayout{
                relative_1 : Vec2 { x: 35.0, y: 40.0 },
                relative_2 : Vec2 { x: 65.0, y: 60.0 },
                ..Default::default()
            }
        ).unwrap();
    button_section(&mut commands, &asset_server, &mut ui, button_base);

    // add ui tree to ecs
    commands.spawn((ui, MainUI));
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn deselect_client_if_pending(
    mut commands      : Commands,
    mut current_state : Query<(&mut PendingSelect, &Callback<Deselect>)>,
){
    let (mut pending_select, deselect_callback) = current_state.single_mut();
    if !pending_select.0.is_some() { return; };
    commands.add(deselect_callback.clone());
    pending_select.0 = None;
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn update_connection_status(
    mut commands : Commands,
    client       : Res<DemoClient>,
    mut status   : ResMut<ConnectionStatus>
){
    while let Some(connection_report) = client.next_report()
    {
        match connection_report
        {
            bevy_simplenet::ClientReport::Connected         => *status = ConnectionStatus::Connected,
            bevy_simplenet::ClientReport::Disconnected      |
            bevy_simplenet::ClientReport::ClosedByServer(_) |
            bevy_simplenet::ClientReport::ClosedBySelf      =>
            {
                *status = ConnectionStatus::Connecting;
                commands.add(|world: &mut World| syscall(world, (), deselect_client_if_pending));
            }
            bevy_simplenet::ClientReport::IsDead => *status = ConnectionStatus::Dead,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_server_incoming(
    mut commands      : Commands,
    client            : Res<DemoClient>,
    mut current_state : Query<(&mut PendingSelect, &Callback<Deselect>)>,
){
    let (mut pending_select, deselect_callback) = current_state.single_mut();

    while let Some(DemoServerVal::Msg(message)) = client.next_val()
    {
        match message
        {
            DemoServerMsg::AckSelect =>
            {
                pending_select.0 = None;
            }
            DemoServerMsg::Deselect =>
            {
                // ignore deselects if the result of our most recent select is unknown
                if pending_select.0.is_some() { continue; }
                commands.add(deselect_callback.clone());
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn check_pending_select(
    mut commands  : Commands,
    current_state : Query<&PendingSelect>,
){
    let pending_select = current_state.single();
    let Some(message_signal) = &pending_select.0 else { return; };

    match message_signal.status()
    {
        bevy_simplenet::MessageStatus::Sending => (),
        bevy_simplenet::MessageStatus::Sent    => (),  //don't clear it yet, wait for ack or disconnect
        bevy_simplenet::MessageStatus::Failed  =>
        {
            commands.add(|world: &mut World| syscall(world, (), deselect_client_if_pending));
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    // simplenet client
    // - we use a baked-in address so you can close and reopen the server to test clients being disconnected
    let client = client_factory().new_client(
            enfync::builtin::Handle::default(),  //automatically selects native/WASM runtime
            url::Url::parse("ws://127.0.0.1:48888/ws").unwrap(),
            bevy_simplenet::AuthRequest::None{ client_id: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() },
            bevy_simplenet::ClientConfig{
                reconnect_on_disconnect   : true,
                reconnect_on_server_close : true,
                ..Default::default()
            },
            ()
        );

    // run client
    App::new()
        .add_plugins(
            bevy::DefaultPlugins.set(
                WindowPlugin{
                    primary_window: Some(Window{ window_theme: Some(WindowTheme::Dark), ..Default::default() }),
                    ..Default::default()
                }
            )
        )
        .add_plugins(LunexUiPlugin)
        .register_interaction_source(MouseLButtonMain::default())
        .insert_resource(client)
        .insert_resource(ConnectionStatus::Connecting)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, update_connection_status)
        .add_systems(Update,
            (
                handle_server_incoming,
                check_pending_select,
                refresh_status_text,
            ).chain()
        )
        .run();
}

//-------------------------------------------------------------------------------------------------------------------
