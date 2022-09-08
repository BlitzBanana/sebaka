use bevy::{prelude::*, render::camera::RenderTarget};
use bevy_pancam::{PanCamPlugin, PanCam};
use bevy_prototype_debug_lines::*;
use heron::*;

fn main() {
    App::new()
        .insert_resource(Gravity::from(Vec3::new(0., 0., 0.)))
        .add_plugins(DefaultPlugins)
        .add_plugin(PanCamPlugin::default())
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(PhysicsPlugin::default())
        .add_startup_system(setup)
        .add_system(movement)
        .add_system(seek_movement_marker)
        .add_system(track_mouse)
        .add_system(move_movement_marker_on_click)
        .add_system(debug_velocity.chain(debug_acceleration))
        .add_system(debug_movement_marker)
        .run();
}

#[derive(Default)]
struct MouseScreenPosition(Option<Vec2>);

#[derive(Default)]
struct MouseWorldPosition(Option<Vec2>);

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
    commands.spawn()
        .insert_bundle(Camera2dBundle::default())
        .insert(MainCamera)
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Left],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 1.,
            max_scale: Some(40.),
            ..default()
        });

    // Spawn the controllable spaceship
    commands.spawn()
        .insert(RigidBody::Dynamic)
        .insert(Velocity::from_linear(Vec3::ZERO))
        .insert(MaxVelocity(500.))
        .insert(Acceleration::from_linear(Vec3::ZERO))
        .insert(MaxAcceleration(10.))
        .insert(CollisionShape::Capsule {
            radius: 50.0,
            half_segment: 50.0,
        })
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("ship.png"),
            ..default()
        });

    // Spawn the movement marker, one and only one !
    commands.spawn()
        .insert_bundle((Transform::default(), GlobalTransform::default()))
        .insert(MovementMarker);

    // Spawn resources related to mouse position tracking
    commands.insert_resource(MouseScreenPosition(None));
    commands.insert_resource(MouseWorldPosition(None));
}

/// Update position according to velocity and velocity according to acceleration
fn movement(mut query: Query<(&mut Transform, &mut Velocity, &Acceleration)>, time: Res<Time>) {
    for (mut transform, mut velocity, acceleration) in query.iter_mut() {
        velocity.linear += acceleration.linear * time.delta_seconds();
        transform.translation += velocity.linear * time.delta_seconds();
    }
}

/// Update acceleration according to movement marker position
fn seek_movement_marker(
    mut query: Query<(&Transform, &Velocity, Option<&MaxVelocity>, &mut Acceleration, Option<&MaxAcceleration>)>,
    target_query: Query<&Transform, With<MovementMarker>>,
) {
    let target_tranform = target_query.single();

    for (transform, velocity, max_velocity, mut acceleration, max_acceleration) in query.iter_mut() {
        let difference = target_tranform.translation - transform.translation;
        let distance = difference.length_squared();

        acceleration.linear = if distance > f32::EPSILON {
            let max_velocity = max_velocity.map(|m| m.0).unwrap_or(f32::INFINITY);
            let max_acceleration = max_acceleration.map(|m| m.0).unwrap_or(f32::INFINITY);
            (difference.normalize_or_zero() * max_velocity - velocity.linear).clamp_length_max(max_acceleration)
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
        target_tranform.translation = mouse_world_position.0
            .map(|m| m.extend(0.))
            .unwrap_or(target_tranform.translation);

        println!("Moved target to {:?}", target_tranform.translation);
    }
}

/// Update mouse tracking related resources
fn track_mouse(
    windows: Res<Windows>,
    query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mouse_screen_coords: ResMut<MouseScreenPosition>,
    mut mouse_world_coords: ResMut<MouseWorldPosition>
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
        mouse_world_coords.0 = Some(world_pos);
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
        lines.line_colored(start, end, 0., Color::BLUE);
    }
}

/// Draw a crosshair on MovementMarker position
fn debug_movement_marker(target_query: Query<&Transform, With<MovementMarker>>, mut lines: ResMut<DebugLines>) {
    let target_tranform = target_query.single();
    lines.line_colored(target_tranform.translation + Vec3::NEG_X * 10., target_tranform.translation + Vec3::X * 10., 0., Color::RED);
    lines.line_colored(target_tranform.translation + Vec3::NEG_Y * 10., target_tranform.translation + Vec3::Y * 10., 0., Color::RED);
}
