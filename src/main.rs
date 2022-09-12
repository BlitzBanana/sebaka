use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget, render_resource::WgpuFeatures, settings::WgpuSettings,
        texture::ImageSettings,
    },
    window::WindowMode,
};
use bevy_hanabi::*;
use bevy_kira_audio::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_debug_lines::*;
use heron::*;
use std::f32::consts::PI;
use steering::SteeringBehaviour;

mod steering;

fn main() {
    let window = WindowDescriptor {
        title: "Sebaka".to_string(),
        mode: WindowMode::BorderlessFullscreen,
        ..Default::default()
    };

    let mut options = WgpuSettings::default();
    options
        .features
        .set(WgpuFeatures::VERTEX_WRITABLE_STORAGE, true);

    App::new()
        .insert_resource(window)
        .insert_resource(options)
        .insert_resource(ImageSettings::default_nearest())
        .insert_resource(Gravity::from(Vec3::new(0., 0., 0.)))
        .insert_resource(ClearColor(Color::rgb(0.0196, 0.0235, 0.0235)))
        .insert_resource(MouseScreenPosition(None))
        .insert_resource(MouseWorldPosition(None))
        .add_plugins(DefaultPlugins)
        .add_plugin(AudioPlugin)
        .add_plugin(PanCamPlugin::default())
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(HanabiPlugin)
        .add_startup_system(setup)
        .add_startup_system(start_ambient_music)
        .add_system(orientation)
        .add_system(thruster_power)
        .add_system(steering_behaviour)
        // .add_system(arrive_to_movement_marker)
        .add_system(track_mouse)
        .add_system(move_movement_marker_on_click)
        .add_system(debug_velocity.chain(debug_acceleration))
        .add_system(debug_movement_marker)
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Default)]
struct MouseScreenPosition(Option<Vec2>);

#[derive(Default)]
struct MouseWorldPosition(Option<Vec3>);

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct MovementMarker;

#[derive(Component)]
struct Spaceship;

#[derive(Component)]
struct ThrusterEffect {
    size: f32,
    angle: f32,
}

#[derive(Component)]
struct MaxVelocity(f32);

#[derive(Component)]
struct MaxAcceleration(f32);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    // Spawn the main camera
    commands
        .spawn()
        .insert_bundle(Camera2dBundle::default())
        .insert(MainCamera)
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Left],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 0.01,
            max_scale: Some(40.),
        });

    // Spawn the movement marker, one and only one !
    let movement_marker = commands
        .spawn()
        .insert_bundle(TransformBundle::default())
        .insert(MovementMarker)
        .id();

    // Spawn the controllable spaceship
    commands
        .spawn()
        .insert_bundle(TransformBundle::default())
        .insert(Spaceship)
        .insert(RigidBody::Dynamic)
        .insert(Velocity::from_linear(Vec3::ZERO))
        .insert(Acceleration::from_linear(Vec3::ZERO))
        .insert(SteeringBehaviour::Seek {
            target: movement_marker,
        })
        .insert(CollisionShape::Capsule {
            radius: 100.0,
            half_segment: 25.0,
        })
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("ship666.png"),
            ..default()
        })
        .with_children(|builder| {
            let main_effect = effects.add(
                EffectAsset {
                    name: "thruster".into(),
                    capacity: 32768,
                    spawner: Spawner::rate(1000.0.into()),
                    ..Default::default()
                }
                .init(PositionCone3dModifier {
                    speed: 250.0.into(),
                    dimension: ShapeDimension::Volume,
                    base_radius: 25.,
                    top_radius: 1.,
                    height: 50.,
                })
                .init(ParticleLifetimeModifier { lifetime: 1.5 })
                .render(SizeOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec2::splat(6.8));
                        gradient.add_key(0.05, Vec2::splat(4.5));
                        gradient.add_key(0.10, Vec2::splat(1.2));
                        gradient.add_key(0.15, Vec2::splat(0.2));
                        gradient.add_key(0.25, Vec2::splat(8.5));
                        gradient.add_key(1.00, Vec2::splat(0.5));
                        gradient
                    },
                })
                .render(ColorOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec4::new(1.0, 0.8, 0.3, 1.0));
                        gradient.add_key(0.03, Vec4::new(1.0, 0.66, 0.0, 1.0));
                        gradient.add_key(0.10, Vec4::new(1.0, 0.55, 0.0, 0.8));
                        gradient.add_key(0.15, Vec4::new(0.0, 0.0, 0.0, 0.0));
                        gradient.add_key(0.25, Vec4::new(0.56, 0.52, 0.51, 0.8));
                        gradient.add_key(1.00, Vec4::new(0.56, 0.52, 0.51, 0.0));
                        gradient
                    },
                }),
            );

            let mut transform = Transform::default();
            transform.rotation = Quat::from_axis_angle(Vec3::Z, PI);
            transform.translation = Vec3::new(0., -160., 0.);

            builder
                .spawn_bundle(ParticleEffectBundle {
                    // Assign the Z layer so it appears in the egui inspector and can be modified at runtime
                    effect: ParticleEffect::new(main_effect).with_z_layer_2d(Some(0.1)),
                    transform,
                    ..default()
                })
                .insert(ThrusterEffect {
                    size: 1.0,
                    angle: PI,
                });

            let secondary_effect = effects.add(
                EffectAsset {
                    name: "thruster".into(),
                    capacity: 32768,
                    spawner: Spawner::rate(1000.0.into()),
                    ..Default::default()
                }
                .init(PositionCone3dModifier {
                    speed: 250.0.into(),
                    dimension: ShapeDimension::Volume,
                    base_radius: 5.,
                    top_radius: 1.,
                    height: 50.,
                })
                .init(ParticleLifetimeModifier { lifetime: 1.5 })
                .render(SizeOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec2::splat(6.8));
                        gradient.add_key(0.05, Vec2::splat(4.5));
                        gradient.add_key(0.10, Vec2::splat(1.2));
                        gradient.add_key(0.15, Vec2::splat(0.2));
                        gradient.add_key(0.25, Vec2::splat(8.5));
                        gradient.add_key(1.00, Vec2::splat(0.5));
                        gradient
                    },
                })
                .render(ColorOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec4::new(1.0, 0.8, 0.3, 1.0));
                        gradient.add_key(0.03, Vec4::new(1.0, 0.66, 0.0, 1.0));
                        gradient.add_key(0.10, Vec4::new(1.0, 0.55, 0.0, 0.8));
                        gradient.add_key(0.15, Vec4::new(0.0, 0.0, 0.0, 0.0));
                        gradient.add_key(0.25, Vec4::new(0.56, 0.52, 0.51, 0.8));
                        gradient.add_key(1.00, Vec4::new(0.56, 0.52, 0.51, 0.0));
                        gradient
                    },
                }),
            );

            let mut transform = Transform::default();
            transform.rotation = Quat::from_axis_angle(Vec3::Z, 0.);
            transform.translation = Vec3::new(-50., 205., 0.);

            builder
                .spawn_bundle(ParticleEffectBundle {
                    // Assign the Z layer so it appears in the egui inspector and can be modified at runtime
                    effect: ParticleEffect::new(secondary_effect).with_z_layer_2d(Some(0.1)),
                    transform,
                    ..default()
                })
                .insert(ThrusterEffect {
                    size: 0.4,
                    angle: 0.,
                });

            let secondary_effect = effects.add(
                EffectAsset {
                    name: "thruster".into(),
                    capacity: 32768,
                    spawner: Spawner::rate(1000.0.into()),
                    ..Default::default()
                }
                .init(PositionCone3dModifier {
                    speed: 250.0.into(),
                    dimension: ShapeDimension::Volume,
                    base_radius: 5.,
                    top_radius: 1.,
                    height: 50.,
                })
                .init(ParticleLifetimeModifier { lifetime: 1.5 })
                .render(SizeOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec2::splat(6.8));
                        gradient.add_key(0.05, Vec2::splat(4.5));
                        gradient.add_key(0.10, Vec2::splat(1.2));
                        gradient.add_key(0.15, Vec2::splat(0.2));
                        gradient.add_key(0.25, Vec2::splat(8.5));
                        gradient.add_key(1.00, Vec2::splat(0.5));
                        gradient
                    },
                })
                .render(ColorOverLifetimeModifier {
                    gradient: {
                        let mut gradient = Gradient::new();
                        gradient.add_key(0.00, Vec4::new(1.0, 0.8, 0.3, 1.0));
                        gradient.add_key(0.03, Vec4::new(1.0, 0.66, 0.0, 1.0));
                        gradient.add_key(0.10, Vec4::new(1.0, 0.55, 0.0, 0.8));
                        gradient.add_key(0.15, Vec4::new(0.0, 0.0, 0.0, 0.0));
                        gradient.add_key(0.25, Vec4::new(0.56, 0.52, 0.51, 0.8));
                        gradient.add_key(1.00, Vec4::new(0.56, 0.52, 0.51, 0.0));
                        gradient
                    },
                }),
            );

            let mut transform = Transform::default();
            transform.rotation = Quat::from_axis_angle(Vec3::Z, 0.);
            transform.translation = Vec3::new(50., 205., 0.);

            builder
                .spawn_bundle(ParticleEffectBundle {
                    // Assign the Z layer so it appears in the egui inspector and can be modified at runtime
                    effect: ParticleEffect::new(secondary_effect.clone())
                        .with_z_layer_2d(Some(0.1)),
                    transform,
                    ..default()
                })
                .insert(ThrusterEffect {
                    size: 0.4,
                    angle: 0.,
                });
        });

    // Spawn some asteroids
    // commands
    //     .spawn()
    //     .insert(RigidBody::Static)
    //     .insert(CollisionShape::Sphere { radius: 105. })
    //     .insert_bundle(SpriteBundle {
    //         texture: asset_server.load("asteroid.png"),
    //         transform: Transform::from_translation(Vec3::new(0., 2000., 0.)),
    //         ..default()
    //     });
}

fn start_ambient_music(asset_server: Res<AssetServer>, audio: Res<bevy_kira_audio::Audio>) {
    audio
        .play(asset_server.load("ambient.ogg"))
        .looped()
        .with_volume(0.3);
}

/// Update orientation according to velocity vector (not really the desired behaviour, but it will do for now)
fn orientation(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        if velocity.linear.length_squared() > f32::EPSILON {
            let angle = {
                // Use Vec2 because we cannot tell apart clockwise 🕓 and anti-clockwise 🕗 angles in 3D
                let angle = Vec2::Y.angle_between(velocity.linear.truncate());
                // The delta angle can be negative (anti-clockwise), in this case we should add one complete turn (2PI) to get back a clockwise angle
                if angle.is_sign_negative() {
                    2. * PI + angle // Anti-clockwise to Clockwise
                } else {
                    angle // Already clockwise
                }
            };
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

/// A dumb system to make thruster particule emiter rate match acceleration of ships
fn thruster_power(
    q_spaceship: Query<
        (
            &Transform,
            &Acceleration,
            Option<&MaxAcceleration>,
            &Children,
        ),
        With<Spaceship>,
    >,
    mut q_thruster: Query<(&mut ParticleEffect, &ThrusterEffect)>,
) {
    for (&transform, &acceleration, max_acceleration, children) in &q_spaceship {
        for &child in children {
            if let Ok((mut effect, thruster)) = q_thruster.get_mut(child) {
                let current_acceleration = acceleration.linear.length();
                let max_acceleration = max_acceleration
                    .map(|m| m.0)
                    .unwrap_or(current_acceleration);
                let current_power = current_acceleration / max_acceleration;

                // UP is 0, LEFT is PI/2, DOWN is PI, RIGHT is 3/2PI, UP is 2PI
                let ship_angle = transform.rotation.to_axis_angle().1;
                let accel_angle = Vec2::Y.angle_between(acceleration.linear.truncate()) + PI;
                let accel_angle = (2. * PI - (ship_angle - accel_angle).abs())
                    .min((ship_angle - accel_angle).abs());
                let alignement =
                    (1. / (accel_angle / PI - thruster.angle / PI).abs() - 1.5).clamp(0., 5.);

                effect.set_spawner(Spawner::rate(
                    (current_power * alignement * thruster.size * 200.).into(),
                ))
            }
        }
    }
}

/// Update acceleration according to movement marker position
fn steering_behaviour(
    mut query: Query<(
        &SteeringBehaviour,
        &Transform,
        &Velocity,
        Option<&MaxVelocity>,
        &mut Acceleration,
        Option<&MaxAcceleration>,
    )>,
    target_query: Query<&Transform, With<MovementMarker>>,
) {
    for (behaviour, transform, velocity, max_velocity, mut acceleration, max_acceleration) in
        &mut query
    {
        match behaviour {
            SteeringBehaviour::Seek { target } => {
                let target = target_query.get(*target).unwrap();
                acceleration.linear = {
                    let max_velocity = max_velocity.map(|m| m.0).unwrap_or(1000.);
                    let max_acceleration = max_acceleration.map(|m| m.0).unwrap_or(100.);

                    let difference = target.translation - transform.translation;
                    let desired_velocity = difference.normalize_or_zero() * max_velocity;
                    let steering =
                        (desired_velocity - velocity.linear).clamp_length_max(max_acceleration);

                    steering
                };
            }
            SteeringBehaviour::Arrive {
                target,
                final_angle,
            } => {
                let target = target_query.get(*target).unwrap();
                acceleration.linear = {
                    let max_velocity = max_velocity.map(|m| m.0).unwrap_or(1000.);
                    let max_acceleration = max_acceleration.map(|m| m.0).unwrap_or(100.);

                    let difference = target.translation - transform.translation;
                    let desired_velocity = difference.normalize_or_zero() * max_velocity;
                    let steering = (desired_velocity
                        - velocity.linear
                            * (1. + velocity.linear.length() * 10. / difference.length().max(1.)))
                    .clamp_length_max(max_acceleration);

                    steering
                };
            }
            SteeringBehaviour::Persue {
                target,
                min_distance,
            } => todo!(),
            SteeringBehaviour::Flee { target } => todo!(),
            SteeringBehaviour::Evade {
                target,
                min_distance,
            } => todo!(),
            SteeringBehaviour::FollowPath {
                path,
                current_index,
            } => todo!(),
            SteeringBehaviour::Interpose {
                from_target,
                to_target,
            } => todo!(),
            SteeringBehaviour::Hide { target } => todo!(),
        }
    }
}

/// Update acceleration according to movement marker position
fn arrive_to_movement_marker(
    mut query: Query<(
        &Transform,
        &Velocity,
        Option<&MaxVelocity>,
        &mut Acceleration,
        Option<&MaxAcceleration>,
    )>,
    target_query: Query<&Transform, With<MovementMarker>>,
    time: Res<Time>,
) {
    let target_tranform = target_query.single();

    for (transform, velocity, max_velocity, mut acceleration, max_acceleration) in &mut query {
        let difference = target_tranform.translation - transform.translation;
        let distance = difference.length();

        acceleration.linear = if distance > f32::EPSILON {
            let max_velocity = max_velocity.map(|m| m.0).unwrap_or(5000.);
            let max_acceleration = max_acceleration.map(|m| m.0).unwrap_or(200.);
            let speed = velocity.linear.length();
            let stop_distance = speed.powi(2) / (2. * max_acceleration);
            let missalignement =
                (90. - (difference.angle_between(velocity.linear) * (180. / PI) - 90.).abs()).abs()
                    / 90.
                    * 100.;

            (if distance - stop_distance < speed * time.delta_seconds() {
                // Decelerate before it's too late to stop at the target
                println!(
                    "▼ {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                (difference.normalize_or_zero() / 2. - velocity.linear.normalize_or_zero())
                    * max_velocity
            } else if distance < 30. {
                // Kill the velocity, target reached
                println!(
                    "⏹ {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                velocity.linear * -1.
            } else if missalignement > 2.0 {
                // Align with the target if needed
                println!(
                    "🗘 {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                (difference - velocity.linear * 15.).normalize_or_zero() * max_velocity
            } else {
                // Go torward the target as fast as posible
                println!(
                    "▲ {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                difference.normalize_or_zero() * max_velocity - velocity.linear
            })
            .clamp_length_max(max_acceleration)
        } else {
            Vec3::ZERO
        };
    }
}

/// Move the movement marker on mouse right click
fn move_movement_marker_on_click(
    mut target_query: Query<&mut Transform, With<MovementMarker>>,
    mouse_world_position: Res<MouseWorldPosition>,
    buttons: Res<Input<MouseButton>>,
) {
    if buttons.just_released(MouseButton::Right) {
        let mut target_tranform = target_query.single_mut();
        target_tranform.translation = mouse_world_position
            .0
            .unwrap_or(target_tranform.translation);

        println!("Moved target to {:?}", target_tranform.translation);
    }
}

/// Update mouse tracking related resources
fn track_mouse(
    windows: Res<Windows>,
    query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mouse_screen_coords: ResMut<MouseScreenPosition>,
    mut mouse_world_coords: ResMut<MouseWorldPosition>,
) {
    let (camera, camera_transform) = query.single();
    let window = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };

    if let Some(screen_pos) = window.cursor_position() {
        let window_size = Vec2::new(window.width() as f32, window.height() as f32);
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
        let world_pos: Vec2 = world_pos.truncate();

        mouse_screen_coords.0 = Some(screen_pos);
        mouse_world_coords.0 = Some(world_pos.extend(0.));
    } else {
        mouse_screen_coords.0 = None;
        mouse_world_coords.0 = None;
    }
}

fn debug_velocity(query: Query<(&Transform, &Velocity)>, mut lines: ResMut<DebugLines>) {
    for (transform, velocity) in &query {
        let start = transform.translation;
        let end = start + velocity.linear;
        lines.line_colored(start, end, 0., Color::YELLOW);
    }
}

fn debug_acceleration(query: Query<(&Transform, &Acceleration)>, mut lines: ResMut<DebugLines>) {
    for (transform, acceleration) in &query {
        let start = transform.translation;
        let end = start + acceleration.linear;
        lines.line_colored(start, start + (start - end), 0., Color::BLUE);
    }
}

/// Draw a crosshair on MovementMarker position
fn debug_movement_marker(
    target_query: Query<&Transform, With<MovementMarker>>,
    mut lines: ResMut<DebugLines>,
) {
    let target_tranform = target_query.single();
    lines.line_colored(
        target_tranform.translation + Vec3::NEG_X * 10.,
        target_tranform.translation + Vec3::X * 10.,
        0.,
        Color::RED,
    );
    lines.line_colored(
        target_tranform.translation + Vec3::NEG_Y * 10.,
        target_tranform.translation + Vec3::Y * 10.,
        0.,
        Color::RED,
    );
}
