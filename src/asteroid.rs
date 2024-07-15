use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::resource::GlobalEntropy;
use rand_core::RngCore;

use crate::player::{GameLayer, PlayerId, PlayerLinkCollider};
use crate::{AsteroidEffect, GameState, ScoreText};

#[derive(Component)]
struct AsteroidSpawner {
    timer: Timer,
}

#[derive(Component)]
pub struct Asteroid;

#[derive(Bundle)]
struct AsteroidBundle {
    sprite: SpriteBundle,
    collider: Collider,
    sensor: Sensor,
    collision_layer: CollisionLayers,
    rigidbody: RigidBody,
    velocity: LinearVelocity,
    mass: MassPropertiesBundle,
    asteroid: Asteroid,
}

fn setup_spawner(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.3, Vec2::new(0.2, 0.02));
    size_gradient1.add_key(1.0, Vec2::splat(0.0));

    for x in -1..=1 {
        for y in -1..=1 {
            if x == 0 || y == 0 {
                continue;
            }

            let position = Vec3::new((x as f32) * 200.0, (y as f32) * 200.0, 0.0);

            let writer = ExprWriter::new();

            let init_pos = SetPositionCircleModifier {
                center: writer.lit(Vec3::ZERO).expr(),
                axis: writer.lit(Vec3::Z).expr(),
                radius: writer.lit(20.).expr(),
                dimension: ShapeDimension::Surface,
            };

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // Give a bit of variation by randomizing the lifetime per particle
            let lifetime = writer.lit(3.0).uniform(writer.lit(4.0)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Add drag to make particles slow down a bit after the initial acceleration
            let drag = writer.lit(15.).expr();
            let update_drag = LinearDragModifier::new(drag);
            let mut module = writer.finish();
            let tangent_accel =
                TangentAccelModifier::constant(&mut module, position, Vec3::Y, 300.);

            let effect = effects.add(
                EffectAsset::new(vec![16384, 16384], Spawner::rate(5000.0.into()), module)
                    .with_name("portal")
                    .init(init_pos)
                    .init(init_age)
                    .init(init_lifetime)
                    .update(update_drag)
                    .update(tangent_accel)
                    .render(ColorOverLifetimeModifier {
                        gradient: color_gradient1.clone(),
                    })
                    .render(SizeOverLifetimeModifier {
                        gradient: size_gradient1.clone(),
                        screen_space_size: false,
                    })
                    .render(OrientModifier::new(OrientMode::AlongVelocity)),
            );

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(35.0)),
                        color: Color::srgba(0.5, 0.4, 0.6, 1.0),
                        ..default()
                    },
                    transform: Transform::from_xyz((x as f32) * 200.0, (y as f32) * 200.0, -1.0),
                    texture: asset_server.load("textures/spawner.png"),
                    ..default()
                },
                AsteroidSpawner {
                    timer: Timer::new(Duration::from_secs_f32(3.0), TimerMode::Repeating),
                },
                ParticleEffect::new(effect.clone()),
                CompiledParticleEffect::default(),
                EffectProperties::default(),
                // ParticleEffectBundle {
                //     transform: Transform::from_translation(position),
                //     ..default()
                // },
            ));
        }
    }
}

#[derive(Component)]
struct HealthBar(f32);

#[derive(Component)]
pub struct HealthParent;

const MAX_SIZE_HEALTHBAR: f32 = 180.0;

fn setup_health_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(175.0),
                    height: Val::Px(50.0),
                    left: Val::Px(40.0),
                    top: Val::Px(40.0),
                    ..default()
                },
                ..default()
            },
            HealthParent,
        ))
        .with_children(|parent| {
            parent.spawn((
                ImageBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(MAX_SIZE_HEALTHBAR),
                        height: Val::Px(50.0),
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    image: asset_server.load("textures/ui/health.png").into(),
                    ..default()
                },
                HealthBar(1.0),
            ));
            parent.spawn((
                ImageBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(MAX_SIZE_HEALTHBAR),
                        height: Val::Px(50.0),
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    image: asset_server.load("textures/ui/transparent.png").into(),
                    ..default()
                },
                Outline {
                    width: Val::Px(6.0),
                    offset: Val::Px(0.0),
                    color: Color::WHITE,
                },
            ));
        });
}

fn update_health_ui(mut health_bar_query: Query<(&mut Style, &HealthBar), Changed<HealthBar>>) {
    let Ok((mut health_style, health_bar)) = health_bar_query.get_single_mut() else {
        return;
    };
    health_style.width = Val::Px(MAX_SIZE_HEALTHBAR * health_bar.0);
}

fn asteroid_spawner(
    mut spawner: Query<(&mut AsteroidSpawner, &Transform)>,
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rng: ResMut<GlobalEntropy<WyRand>>,
) {
    for (mut spawner_timer, spawner_trans) in spawner.iter_mut() {
        spawner_timer.timer.tick(time.delta());

        if spawner_timer.timer.finished() {
            let new_timer_duration = spawner_timer
                .timer
                .duration()
                .mul_f32(0.96)
                .clamp(Duration::from_secs_f32(0.5), Duration::MAX);
            spawner_timer.timer.set_duration(new_timer_duration);
            spawner_timer.timer.reset();

            let random_dir = Vec2 {
                x: 2.0 * (rng.next_u32() as f32) / (u32::MAX as f32) - 1.0,
                y: 2.0 * (rng.next_u32() as f32) / (u32::MAX as f32) - 1.0,
            }
            .normalize();

            const MAX_ASTEROID_SPEED: f32 = 150.0;
            const MIN_ASTEROID_SPEED: f32 = 100.0;
            let random_01 = (rng.next_u32() as f32) / (u32::MAX as f32);
            let random_speed =
                (random_01 * (MAX_ASTEROID_SPEED - MIN_ASTEROID_SPEED)) + MIN_ASTEROID_SPEED;
            let random_size = 30.0 + random_01 * 5.0;

            commands.spawn(AsteroidBundle {
                sprite: SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2 {
                            x: random_size,
                            y: random_size,
                        }),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        spawner_trans.translation.x,
                        spawner_trans.translation.y,
                        0.0,
                    ),
                    texture: asset_server.load("textures/asteroid.png"),
                    ..default()
                },
                collider: Collider::circle(random_size / 2.0 * 0.8),
                sensor: Sensor,
                collision_layer: CollisionLayers::new(
                    GameLayer::Asteroid,
                    [GameLayer::Player, GameLayer::Link],
                ),

                rigidbody: RigidBody::Dynamic,
                velocity: LinearVelocity(random_dir * random_speed),
                mass: MassPropertiesBundle::new_computed(&Collider::circle(1.0), 1.0),
                asteroid: Asteroid,
            });
        }
    }
}

fn asteroid_trigger(
    mut commands: Commands,
    mut collision_event_reader: EventReader<Collision>,
    asteroids_q: Query<
        (Entity, &Transform),
        (
            Without<PlayerLinkCollider>,
            Without<PlayerId>,
            With<Asteroid>,
            Without<HealthBar>,
            Without<ScoreText>,
        ),
    >,
    players_q: Query<
        Entity,
        (
            Without<PlayerLinkCollider>,
            With<PlayerId>,
            Without<Asteroid>,
            Without<HealthBar>,
            Without<ScoreText>,
        ),
    >,
    links_q: Query<
        Entity,
        (
            With<PlayerLinkCollider>,
            Without<PlayerId>,
            Without<Asteroid>,
            Without<HealthBar>,
            Without<ScoreText>,
        ),
    >,
    mut health_q: Query<
        &mut HealthBar,
        (
            Without<PlayerLinkCollider>,
            Without<PlayerId>,
            Without<Asteroid>,
            With<HealthBar>,
            Without<ScoreText>,
        ),
    >,
    mut score_q: Query<
        &mut ScoreText,
        (
            Without<PlayerLinkCollider>,
            Without<PlayerId>,
            Without<Asteroid>,
            Without<HealthBar>,
            Without<HealthBar>,
        ),
    >,
    mut end_state: ResMut<NextState<GameState>>,
    asteroid_effect: Res<AsteroidEffect>,
) {
    for Collision(contacts) in collision_event_reader.read() {
        let ((asteroid, asteroid_trans), other) = match (
            asteroids_q.get(contacts.entity1),
            asteroids_q.get(contacts.entity2),
        ) {
            (Ok(a), Err(_)) => (a, contacts.entity2),
            (Err(_), Ok(a)) => (a, contacts.entity1),
            _ => continue,
        };

        let player = players_q.get(other).ok();
        let link = links_q.get(other).ok();

        match (link, player) {
            (None, Some(_)) => {
                let Ok(mut health_bar) = health_q.get_single_mut() else {
                    continue;
                };

                health_bar.0 = (health_bar.0 - 0.2).clamp(0.0, 1.0);
                commands.entity(asteroid).despawn_recursive();
                if health_bar.0 == 0.0 {
                    end_state.set(GameState::EndGame);
                }
            }
            (Some(_), None) => {
                commands.spawn(ParticleEffectBundle {
                    effect: ParticleEffect::new(asteroid_effect.0.clone()),
                    transform: Transform::from_translation(asteroid_trans.translation),
                    ..default()
                });
                commands.entity(asteroid).despawn_recursive();
                score_q.iter_mut().for_each(|mut s| s.0 += 1);
            }
            (_, _) => continue,
        }
    }
}

pub struct AsteroidPlugin;

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_spawner)
            .add_systems(Startup, setup_health_ui)
            .add_systems(Update, update_health_ui)
            .add_systems(Update, asteroid_trigger)
            .add_systems(Update, asteroid_spawner);
    }
}
