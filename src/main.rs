use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{camera::RenderTarget, texture::ImageSettings},
    window::WindowMode,
};
use bevy_kira_audio::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_debug_lines::*;
use heron::*;

fn main() {
    let window = WindowDescriptor {
        title: "Sebaka".to_string(),
        mode: WindowMode::BorderlessFullscreen,
        ..Default::default()
    };

    App::new()
        .insert_resource(window)
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
        .add_startup_system(setup)
        .add_startup_system(start_ambient_music)
        // .add_system(orientation)
        .add_system(arrive_to_movement_marker)
        .add_system(track_mouse)
        .add_system(move_movement_marker_on_click)
        .add_system(debug_velocity.chain(debug_acceleration))
        .add_system(debug_movement_marker)
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
struct MaxVelocity(f32);

#[derive(Component)]
struct MaxAcceleration(f32);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn the main camera
    commands
        .spawn()
        .insert_bundle(Camera2dBundle::default())
        .insert(MainCamera)
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Left],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 1.,
            max_scale: Some(40.),
        });

    // Spawn the controllable spaceship
    commands
        .spawn()
        .insert(RigidBody::Dynamic)
        .insert(Velocity::from_linear(Vec3::ZERO))
        // .insert(MaxVelocity(2000.))
        .insert(Acceleration::from_linear(Vec3::ZERO))
        // .insert(MaxAcceleration(25.))
        .insert(CollisionShape::Capsule {
            radius: 100.0,
            half_segment: 25.0,
        })
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("ship101.png"),
            ..default()
        });

    // Spawn some asteroids
    commands
        .spawn()
        .insert(RigidBody::Static)
        .insert(CollisionShape::Sphere { radius: 105. })
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("asteroid.png"),
            transform: Transform::from_translation(Vec3::new(0., 2000., 0.)),
            ..default()
        });

    // Spawn the movement marker, one and only one !
    commands
        .spawn()
        .insert_bundle(TransformBundle::default())
        .insert(MovementMarker);
}

fn start_ambient_music(asset_server: Res<AssetServer>, audio: Res<bevy_kira_audio::Audio>) {
    audio
        .play(asset_server.load("ambient.ogg"))
        .looped()
        .with_volume(0.3);
}

/// Update orientation according to velocity vector (not really the desired behaviour, but it will do for now)
fn orientation(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
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

    for (transform, velocity, max_velocity, mut acceleration, max_acceleration) in query.iter_mut()
    {
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
            } else if missalignement > 2. {
                println!(
                    "🗘 {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                (difference - velocity.linear * 15.).normalize_or_zero() * max_velocity
            } else if distance < 30. {
                // Kill the velocity, target reached
                println!(
                    "⏹ {:05.0}m/s, {:04.0}m/s-2, {:03.0}°",
                    speed,
                    acceleration.linear.length(),
                    missalignement
                );
                velocity.linear * -1.
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
    for (transform, velocity) in query.iter() {
        let start = transform.translation;
        let end = start + velocity.linear;
        lines.line_colored(start, end, 0., Color::YELLOW);
    }
}

fn debug_acceleration(query: Query<(&Transform, &Acceleration)>, mut lines: ResMut<DebugLines>) {
    for (transform, acceleration) in query.iter() {
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
