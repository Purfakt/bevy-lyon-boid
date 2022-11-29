use bevy::{prelude::*, render::camera::RenderTarget, window::PresentMode};
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Flock".to_string(),
                width: 800.0,
                height: 800.0,
                present_mode: PresentMode::AutoVsync,
                ..default()
            },
            ..default()
        }))
        .add_plugin(ShapePlugin)
        .add_startup_system(setup_camera)
        .add_startup_system(spawn_target)
        .add_startup_system(spawn_boid)
        .add_system(physics_system)
        .add_system(seek_target)
        .add_system(move_target)
        .add_system(steering.after(physics_system))
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    commands.spawn(Camera2dBundle::default()).insert(MainCamera);
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Boid;

#[derive(Component, Default)]
struct Physics {
    velocity: Vec3,
    acceleration: Vec3,
    max_speed: f32,
    max_force: f32,
}

#[derive(Component, Default)]
struct Steering {
    target: Vec3,
}

#[derive(Component)]
struct Target;

fn spawn_target(mut commands: Commands) {
    let shape = shapes::Rectangle {
        extents: Vec2 { x: 10., y: 10. },
        ..Default::default()
    };

    commands
        .spawn(GeometryBuilder::build_as(
            &shape,
            DrawMode::Outlined {
                fill_mode: FillMode::color(Color::RED),
                outline_mode: StrokeMode::new(Color::WHITE, 1.),
            },
            Transform::from_xyz(0., 0., 10.),
        ))
        .insert(Target);
}

fn spawn_boid(mut commands: Commands) {
    let triangle = shapes::Polygon {
        points: vec![
            Vec2::new(-15., -25.),
            Vec2::new(15., -25.),
            Vec2::new(0., 25.),
        ],
        closed: true,
    };
    let line = shapes::Line(Vec2::new(0., 0.), Vec2::new(0., 50.));

    commands
        .spawn(GeometryBuilder::new().add(&triangle).add(&line).build(
            DrawMode::Outlined {
                fill_mode: FillMode::color(Color::BLUE),
                outline_mode: StrokeMode::new(Color::WHITE, 1.),
            },
            Transform::from_xyz(200., 0., 100.),
        ))
        .insert(Physics {
            velocity: Vec3::new(10., -10., 0.),
            acceleration: Vec3::default(),
            max_speed: 2.,
            max_force: 0.1,
        })
        .insert(Steering {
            target: Vec3::new(0., 0., 0.),
        })
        .insert(Boid);
}

fn physics_system(mut query: Query<(&mut Transform, &mut Physics, With<Boid>)>) {
    for (mut transform, mut physics, _) in query.iter_mut() {
        let previous_acceleration = physics.acceleration;
        let previous_velocity = physics.velocity;
        let previous_position = transform.translation;
        let max_speed = physics.max_speed;

        let new_velocity = previous_velocity + previous_acceleration;
        let new_position = previous_position + new_velocity;

        let angle_between_positions = angle_to_direction(&new_velocity);

        transform.translation = new_position;
        transform.rotation = Quat::from_rotation_z(angle_between_positions);
        physics.velocity = new_velocity.clamp_length_max(max_speed);

        physics.acceleration = Vec3::ZERO;
    }
}

fn angle_to_direction(new_velocity: &Vec3) -> f32 {
    if *new_velocity == Vec3::ZERO {
        0.
    } else {
        new_velocity.angle_between(Vec3::Y) * -new_velocity.x.signum()
    }
}

fn seek_target(
    mut boid_query: Query<(&mut Steering, With<Boid>)>,
    target_query: Query<(&Transform, With<Target>)>,
) {
    let (target, _) = target_query.single();
    for (mut steering, _) in boid_query.iter_mut() {
        steering.target = target.translation;
    }
}

fn move_target(
    // need to get window dimensions
    windows: Res<Windows>,
    // query to get camera transform
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut target_query: Query<(&mut Transform, With<Target>)>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = camera_query.single();

    // get the window that the camera is displaying to (or the primary window)
    let window = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = window.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(window.width() as f32, window.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        let world_pos: Vec2 = world_pos.truncate();
        let mut target = target_query.single_mut().0;

        target.translation = world_pos.extend(0.);
    }
}

fn steering(mut query: Query<(&Transform, &Steering, &mut Physics, With<Boid>)>) {
    for (transform, steering, mut physics, _) in query.iter_mut() {
        let mut desired = steering.target - transform.translation;
        desired = desired.normalize();
        desired = desired * physics.max_speed;

        let steer = (desired - physics.velocity).clamp_length_max(physics.max_force);
        apply_force(physics.as_mut(), &steer);
    }
}

fn apply_force(physics: &mut Physics, force: &Vec3) {
    physics.acceleration = physics.acceleration + *force;
}
