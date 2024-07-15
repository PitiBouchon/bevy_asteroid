use avian2d::prelude::*;
use bevy::input::gamepad::{GamepadConnection, GamepadEvent};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin};
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

#[derive(PhysicsLayer)]
pub enum GameLayer {
    Player,   // Layer 0
    Link,     // Layer 1
    Asteroid, // Layer 2
}

#[derive(Bundle)]
struct PlayerBundle {
    sprite: SpriteBundle,
    player: PlayerId,
    collider: Collider,
    sensor: Sensor,
    collision_layer: CollisionLayers,
    rigidbody: RigidBody,
    mass: MassPropertiesBundle,
    velocity: LinearVelocity,
    damping: LinearDamping,
    locked_axes: LockedAxes,
    gamepad: PlayerGamepad,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct PlayerId(u8);

#[derive(Component)]
pub struct PlayerLink(Entity, Entity, Entity);

#[derive(Component)]
pub struct PlayerLinkCollider;

#[derive(Component)]
struct PlayerGamepad(Option<Gamepad>);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/animate_shader.wgsl".into()
    }
}

fn setup_players(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gamepads: Res<Gamepads>,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let mut gamepads = gamepads.iter().collect::<Vec<_>>();

    const SPAWN_RADIUS: f32 = 100.0;
    let number_players = 2;

    let radius_step = 2.0 * std::f32::consts::PI / (number_players as f32);
    let players_entities = (0..number_players)
        .map(|i| {
            let angle_step = radius_step * (i as f32);
            let pos_x = SPAWN_RADIUS * angle_step.cos();
            let pos_y = SPAWN_RADIUS * angle_step.sin();
            (
                commands
                    .spawn(PlayerBundle {
                        sprite: SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2 { x: 50.0, y: 50.0 }),
                                ..default()
                            },
                            transform: Transform::from_xyz(pos_x, pos_y, 10.0),
                            texture: asset_server.load("textures/ship.png"),
                            ..default()
                        },
                        player: PlayerId(i),
                        collider: Collider::circle(16.0),
                        sensor: Sensor,
                        collision_layer: CollisionLayers::new(
                            GameLayer::Player,
                            [GameLayer::Asteroid],
                        ),
                        rigidbody: RigidBody::Dynamic,
                        gamepad: PlayerGamepad(gamepads.pop()),
                        mass: MassPropertiesBundle::new_computed(&Collider::circle(1.0), 1.0),
                        velocity: LinearVelocity(Vec2::ZERO),
                        damping: LinearDamping(2.0),
                        locked_axes: LockedAxes::ROTATION_LOCKED,
                    })
                    .id(),
                Vec2::new(pos_x, pos_y),
            )
        })
        .collect::<Vec<_>>();

    for [(entity1, pos1), (entity2, pos2)] in players_entities.array_windows::<2>() {
        const LINK_WIDTH: f32 = 10.0;
        let translation = (*pos1 + (*pos2 - *pos1) / 2.0).extend(0.0);
        let rotation = Quat::from_rotation_z(Vec2::Y.angle_between(*pos2 - *pos1));
        let length = pos1.distance(*pos2);
        let player_link_collider = commands
            .spawn((
                TransformBundle {
                    local: Transform {
                        translation,
                        rotation,
                        ..default()
                    },
                    ..default()
                },
                Collider::rectangle(LINK_WIDTH * 0.7, length),
                Sensor,
                CollisionLayers::new(GameLayer::Link, [GameLayer::Asteroid]),
                PlayerLinkCollider,
            ))
            .id();

        commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Rectangle::new(LINK_WIDTH, 1.0))),
                material: materials.add(CustomMaterial {
                    color: LinearRgba::new(50.0, 190.0, 75.0, 1.0),
                }),
                transform: Transform {
                    translation,
                    rotation,
                    scale: Vec2::new(1.0, length).extend(0.0),
                },
                ..default()
            },
            PlayerLink(*entity1, *entity2, player_link_collider),
        ));

        const PLAYER_JOINT_DISTANCE: f32 = 200.0;

        commands.spawn(DistanceJoint {
            entity1: *entity1,
            entity2: *entity2,
            local_anchor1: Vec2::ZERO,
            local_anchor2: Vec2::ZERO,
            rest_length: 0.0,
            length_limits: Some(DistanceLimit {
                min: 0.0,
                max: PLAYER_JOINT_DISTANCE,
            }),
            damping_linear: 10.0,
            damping_angular: 0.0,
            lagrange: 0.0, // TODO: I have no idea what that is
            compliance: 0.01,
            force: Vec2::ONE * 5.0,
        });
    }
}

fn link_follow_players(
    players: Query<
        &Transform,
        (
            With<PlayerId>,
            Without<PlayerLink>,
            Without<PlayerLinkCollider>,
        ),
    >,
    mut player_links: Query<
        (&mut Transform, &PlayerLink),
        (
            Without<PlayerId>,
            With<PlayerLink>,
            Without<PlayerLinkCollider>,
        ),
    >,
    mut player_links_colliders: Query<
        (&mut Transform, &mut Collider),
        (
            Without<PlayerId>,
            Without<PlayerLink>,
            With<PlayerLinkCollider>,
        ),
    >,
) {
    for (mut link_trans, link_info) in player_links.iter_mut() {
        let Ok([player_trans1, player_trans2]) = players.get_many([link_info.0, link_info.1])
        else {
            continue;
        };

        let pos1 = player_trans1.translation.truncate();
        let pos2 = player_trans2.translation.truncate();
        let length = pos1.distance(pos2);
        let translation = (pos1 + (pos2 - pos1) / 2.0).extend(0.0);
        let rotation = Quat::from_rotation_z(Vec2::Y.angle_between(pos2 - pos1));
        link_trans.scale.y = length;

        link_trans.rotation = rotation;

        link_trans.translation = translation;

        let Ok((mut player_link_collider, mut player_link_col)) =
            player_links_colliders.get_mut(link_info.2)
        else {
            continue;
        };

        player_link_collider.translation = translation;
        player_link_collider.rotation = rotation;

        player_link_col.set_scale(Vec2::new(1.0, length), 4);
    }
}

fn gamepad_connect(
    mut players: Query<&mut PlayerGamepad>,
    mut evr_gamepad: EventReader<GamepadEvent>,
) {
    for ev in evr_gamepad.read() {
        let GamepadEvent::Connection(ev_conn) = ev else {
            continue;
        };
        match &ev_conn.connection {
            GamepadConnection::Connected(_info) => {
                for mut player_gamepad in players.iter_mut() {
                    if player_gamepad.0.is_none() {
                        player_gamepad.0 = Some(ev_conn.gamepad);
                        break;
                    }
                }
            }
            GamepadConnection::Disconnected => {
                for mut player_gamepad in players.iter_mut() {
                    if player_gamepad
                        .0
                        .is_some_and(|gamepad| gamepad == ev_conn.gamepad)
                    {
                        player_gamepad.0 = None;
                    }
                }
            }
        }
    }
}

fn gamepad_input(
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    mut players: Query<(&mut Transform, &mut LinearVelocity, &PlayerGamepad)>,
) {
    for (mut player_transform, mut player_velocity, player_gamepad) in players.iter_mut() {
        if let Some(gamepad) = player_gamepad.0 {
            let axis_lx = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickX,
            };
            let axis_ly = GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickY,
            };

            if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
                let left_stick = Vec2::new(x, y);

                const GAMEPAD_DEADZONE: f32 = 0.1;
                if left_stick.length() > GAMEPAD_DEADZONE {
                    player_transform.rotation =
                        Quat::from_rotation_z(left_stick.to_angle() - std::f32::consts::FRAC_PI_2);
                }
            }

            let forward_button = GamepadButton {
                gamepad,
                button_type: GamepadButtonType::South,
            };

            if buttons.pressed(forward_button) {
                const PLAYER_ACCELERATION: f32 = 20.0;
                const MAX_PLAYER_SPEED: f32 = 1000.0;

                player_velocity.0 += Vec2::from_angle(
                    player_transform.rotation.to_scaled_axis().z + std::f32::consts::FRAC_PI_2,
                ) * PLAYER_ACCELERATION;
                player_velocity.0 = player_velocity.clamp_length(0.0, MAX_PLAYER_SPEED);
            }
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_players)
            .add_plugins(Material2dPlugin::<CustomMaterial>::default())
            .add_systems(Update, gamepad_input)
            .add_systems(Update, gamepad_connect)
            .add_systems(Update, link_follow_players);
    }
}
