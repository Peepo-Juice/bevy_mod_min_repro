use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::{ExecutorKind, ScheduleLabel},
    prelude::*,
    window::WindowResolution,
};
use bevy_asset_loader::{
    asset_collection::AssetCollection,
    loading_state::{LoadingState, LoadingStateAppExt, config::ConfigureLoadingState},
    // standard_dynamic_asset::StandardDynamicAssetCollection,
};
use bevy_mod_scripting::{
    BMSPlugin,
    core::{
        ConfigureScriptPlugin,
        asset::ScriptAsset,
        bindings::{
            AppReflectAllocator, FunctionCallContext, GlobalNamespace, NamespaceBuilder,
            ReflectReference, ScriptValue, ThreadWorldContainer, Val, WorldContainer,
        },
        callback_labels,
        commands::{AddStaticScript, RunScriptCallback},
        error::InteropError,
        event::{Recipients, ScriptCallbackEvent},
        handler::event_handler,
    },
    lua::LuaScriptingPlugin,
};

#[derive(AssetCollection, Default, Resource)]
pub struct MyAssets {
    #[asset(path = "startup.lua")]
    pub startup_script: Handle<ScriptAsset>,
    #[asset(path = "test1.lua")]
    pub test1_script: Handle<ScriptAsset>,
}

#[derive(States, Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default)]
pub enum GameState {
    #[default]
    LoadInitialAssets,
    GameRunning,
}

callback_labels!(Start => "start");
callback_labels!(Test => "test");

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BmsSchedule;

// A lot of the stuff I do here was due to me trying to desperately reproduce the error
// so I tried to replicate the details from my game as much as I could
fn main() {
    let mut app = App::new();

    app.add_plugins((bevy::DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: WindowResolution::new(1280.0, 800.0),
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }),));

    let mut custom_schedule = Schedule::new(BmsSchedule);
    custom_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    app.add_schedule(custom_schedule);
    app.world_mut()
        .resource_mut::<MainScheduleOrder>()
        .insert_after(PostUpdate, BmsSchedule);

    app.init_state::<GameState>();
    app.init_resource::<MyAssets>();
    app.add_loading_state(
        LoadingState::new(GameState::LoadInitialAssets)
            .continue_to_state(GameState::GameRunning)
            .load_collection::<MyAssets>(),
    );

    NamespaceBuilder::<GlobalNamespace>::new_unregistered(&mut app.world_mut())
        .register("trigger_event", trigger_lua_event);

    app.add_plugins(
        BMSPlugin.set(LuaScriptingPlugin::default().add_context_initializer(
            |script_id, _context| {
                let world = ThreadWorldContainer
                    .try_get_world()
                    .expect("couldnt get world");

                world.with_global_access(move |w| {
                    w.commands()
                        .queue(AddStaticScript::new(script_id.to_string()));
                })?;

                Ok(())
            },
        )),
    );

    app.add_systems(
        OnEnter(GameState::GameRunning),
        |mut commands: Commands, mut writer: EventWriter<ScriptCallbackEvent>| {
            // writer.write(ScriptCallbackEvent::new(Start, vec![], Recipients::All));

            let new = RunScriptCallback::<LuaScriptingPlugin>::new(
                "startup.lua".into(),
                Entity::from_raw(0),
                Start.into(),
                vec![],
                false,
            );
            commands.queue(new);
        },
    );

    app.add_systems(
        BmsSchedule,
        (
            event_handler::<Start, LuaScriptingPlugin>,
            event_handler::<Test, LuaScriptingPlugin>,
        ),
    );

    app.run();
}

pub fn trigger_lua_event(
    ctx: FunctionCallContext,
    label: String,
    script_id: String,
) -> Result<(), InteropError> {
    Ok(ctx.world()?.with_global_access(|world| {
        println!("trigger_lua_event {:?} {:?}", label, script_id);

        let command = RunScriptCallback::<LuaScriptingPlugin>::new(
            script_id.into(),
            Entity::from_raw(0),
            label.clone().into(),
            Default::default(),
            false,
        );

        world.commands().queue(command);
    })?)
}
