#![feature(array_windows)]
#![allow(clippy::complexity)]

mod asteroid;
mod player;

use asteroid::{AsteroidPlugin, HealthParent};
use avian2d::prelude::*;
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::plugin::EntropyPlugin;
use player::PlayerPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default(), HanabiPlugin))
        // .add_plugins(PhysicsDebugPlugin::default())
        .insert_state(GameState::InGame)
        .add_plugins(PlayerPlugin)
        .add_plugins(AsteroidPlugin)
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .insert_resource(Gravity(Vec2::ZERO))
        .add_systems(Startup, setup_map)
        .add_systems(Startup, setup_score_ui)
        .add_systems(Startup, setup_effects)
        .add_systems(Startup, setup_sound)
        .add_systems(Update, update_time_ui)
        // .add_systems(Update, on_resize)
        .add_systems(OnEnter(GameState::EndGame), end_game)
        .run();
}

#[derive(Component)]
struct Background;

fn setup_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                clear_color: Color::BLACK.into(),
                ..default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
            ..default()
        },
        BloomSettings {
            intensity: 0.05,
            ..default()
        },
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2 {
                    x: 1920.0,
                    y: 1080.0,
                }),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, -100.0),
            texture: asset_server.load("textures/space.png"),
            ..default()
        },
        Background {},
    ));
}

// fn on_resize(
//     mut q: Query<&mut Sprite, With<Background>>,
//     mut resize_reader: EventReader<WindowResized>,
// ) {
//     let mut background_sprite = q.single_mut();
//     for e in resize_reader.read() {
//         background_sprite.custom_size = Some(Vec2::new(e.width, e.height));
//     }
// }

fn end_game(
    mut commands: Commands,
    health_q: Query<Entity, With<HealthParent>>,
    mut time1: ResMut<Time<Physics>>,
    mut time2: ResMut<Time<Virtual>>,
) {
    for e in health_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    commands.spawn(TextBundle {
        style: Style {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
        text: Text::from_section(
            "GAME END",
            TextStyle {
                font_size: 140.0,
                color: Color::srgba(0.9, 0.2, 0.3, 1.0),
                ..default()
            },
        )
        .with_justify(JustifyText::Center),
        ..default()
    });

    time1.pause();
    time2.pause();
}

fn setup_sound(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(AudioBundle {
        source: asset_server.load("music/BossTheme.ogg"),
        ..default()
    });
}

#[derive(Component)]
struct ScoreText(usize);

fn setup_score_ui(mut commands: Commands) {
    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "00",
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                font_size: 60.0,
                ..default()
            },
        ) // Set the justification of the Text
        .with_text_justify(JustifyText::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            align_self: AlignSelf::Start,
            justify_self: JustifySelf::End,
            ..default()
        }),
        ScoreText(0),
    ));
}

fn update_time_ui(mut text_q: Query<(&mut Text, &ScoreText)>) {
    for (mut score_text, score) in &mut text_q.iter_mut() {
        score_text.sections[0].value = format!("{:02}", score.0);
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    InGame,
    EndGame,
}

#[derive(Resource)]
struct AsteroidEffect(Handle<EffectAsset>);

fn setup_effects(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    // Define a color gradient from red to transparent black
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(0.6, 0.5, 0.55, 1.));
    gradient.add_key(1.0, Vec4::splat(0.));

    // Create a new expression module
    let mut module = Module::default();

    // On spawn, randomly initialize the position of the particle
    // to be over the surface of a sphere of radius 2 units.
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(2.0),
        dimension: ShapeDimension::Surface,
    };

    // Also initialize a radial initial velocity to 6 units/sec
    // away from the (same) sphere center.
    let init_vel = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(400.),
    };

    // Initialize the total lifetime of the particle, that is
    // the time for which it's simulated and rendered. This modifier
    // is almost always required, otherwise the particles won't show.
    let lifetime = module.lit(0.2); // literal value "10.0"
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // // Every frame, add a gravity-like acceleration downward
    // let accel = module.lit(Vec3::new(0., -3., 0.));
    // let update_accel = AccelModifier::new(accel);

    // Create the effect asset
    let effect = EffectAsset::new(
        // Maximum number of particles alive at a time
        vec![200],
        // Spawn at a rate of 5 particles per second
        Spawner::once(200.0.into(), true),
        // Move the expression module into the asset
        module,
    )
    .with_name("MyEffect")
    .init(init_pos)
    .init(init_vel)
    .init(init_lifetime)
    // .update(update_accel)
    // Render the particles with a color gradient over their
    // lifetime. This maps the gradient key 0 to the particle spawn
    // time, and the gradient key 1 to the particle death (10s).
    .render(ColorOverLifetimeModifier { gradient });

    // Insert into the asset system
    let effect_handle = effects.add(effect);

    commands.insert_resource(AsteroidEffect(effect_handle));
}
