use bevy::{math::DVec3, prelude::*, render::camera::Camera};
use rand::Rng;
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
struct Star {
    position: DVec3,
    velocity: DVec3,
    acceleration: DVec3,
    mass: f64,
}

#[derive(Default)]
struct CameraAngle(f32);

const G: f64 = 6.674e-11;
const NUMBER_OF_STARS: usize = 1000;
const black_hole_mass: f64 = 0.0;
const star_mass_from: f64 = 1.0e29;
const star_mass_to: f64 = 1.0e32;
const galaxy_diameter: f64 = 1.0e13;
const time_factor: f64 = 1.0e14;
const spin_factor: f64 = 1e-5;
const max_velocity: f64 = 1e-2;
const max_acceleration: f64 = 1e-1;
const min_gravity_distance: f64 = 1.0e1;
const camera_speed: f32 = 0.0;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            vsync: false, // Disabled for this demo to remove vsync as a source of input latency
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(CameraAngle(0.0))
        .add_plugins(DefaultPlugins) // PickingPlugin provides core picking systems and must be registered first
        .add_startup_system(setup.system())
        .add_system(moving.system())
        .add_system(camera_orbit.system())
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // cube
    for i in 0..NUMBER_OF_STARS {
        // create star
        let px = rng.gen_range(-galaxy_diameter..galaxy_diameter);
        let py = rng.gen_range(-galaxy_diameter..galaxy_diameter);
        let pz = rng.gen_range(-galaxy_diameter / 10.0..galaxy_diameter / 10.0);
        let mut star = Star {
            position: if i == 0 {
                DVec3::default()
            } else {
                DVec3::new(px, py, pz)
            },
            velocity: DVec3::default(),
            acceleration: DVec3::default(),
            mass: if i == 0 { black_hole_mass } else { rng.gen_range(star_mass_from..star_mass_to) },
        };

        // spin it
        let angle: f64 = ang::atan2(px, py).in_radians();
        star.acceleration = DVec3::new(angle.cos() * spin_factor, angle.sin() * spin_factor, 0.0);
        star.velocity = star.acceleration;

        commands
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 3.0 })),
                material: materials.add(if i == 0 {
                    Color::rgb(2.0, 2.0, 8.0).into()
                } else {
                    Color::rgb(5.9, 5.9, 1.2).into()
                }),
                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                ..Default::default()
            })
            .insert(star);
    }

    // light
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(-2000.0, 2000.0, 1000.0),
        ..Default::default()
    });

    // camera
    let mut camera = PerspectiveCameraBundle::default();
    camera.transform = Transform::from_xyz(0.0, 0.0, 2500.0);
    camera.transform.look_at(Vec3::ZERO, Vec3::Y);
    camera.perspective_projection.near = 1.0;
    camera.perspective_projection.far = 10000.0;
    commands.spawn_bundle(camera);
}

fn limit_length(v: &mut DVec3, len: f64) {
    if v.length() > len {
        *v = v.normalize() * len;
    }
}

fn moving(time: Res<Time>, mut query: Query<(&mut Star, &mut Transform)>) {
    // based on this code: https://rosettacode.org/wiki/N-body_problem#C.23

    // copy stars to local vector
    let mut stars = Vec::<Star>::new();
    for (star, _) in query.iter_mut() {
        stars.push(star.clone());
    }

    // update accelerations
    for i in 0..stars.len() {
        stars[i].acceleration = DVec3::default();
        for j in 0..stars.len() {
            if i != j {
                let distance = stars[j].position - stars[i].position;
                let distance_length = distance.length();
                if distance_length > min_gravity_distance {
                    let temp = G * stars[j].mass / distance_length.powf(3.0);
                    stars[i].acceleration += distance * temp;
                }
            }
        }
    }

    let time_delta = time.delta().as_secs_f64();
    for i in 0..stars.len() {
        // update velocities
        limit_length(&mut stars[i].acceleration, max_velocity);
        let mut acceleration = stars[i].acceleration;
        stars[i].velocity += acceleration;
        limit_length(&mut stars[i].velocity, max_acceleration);

        // update positions
        let delta = stars[i].velocity + stars[i].acceleration * 0.5;
        stars[i].position += delta * time_delta * time_factor;
        limit_length(&mut stars[i].position, 2.0 * galaxy_diameter);
    }
    stars[0].position = DVec3::default();

    // calculate scale factor and offset to center all starts in a 1000 size box
    let mut min: DVec3 = stars[0].position;
    let mut max: DVec3 = stars[0].position;
    for i in 0..stars.len() {
        let star = stars[i];
        min.x = star.position.x.min(min.x);
        min.y = star.position.y.min(min.y);
        min.z = star.position.z.min(min.z);
        max.x = star.position.x.max(min.x);
        max.y = star.position.y.max(min.y);
        max.z = star.position.z.max(min.z);
    }
    let max_delta = (max.x - min.x).max(max.y - min.y).max(max.z - min.z);
    //let ofs = min + (max - min) * 0.5;
    let ofs = DVec3::default();

    //let scale = 1000.0 / max_delta;
    let scale = 1000.0 / galaxy_diameter;

    // update graphics
    for (i, (mut star, mut transform)) in query.iter_mut().enumerate() {
        let v = (stars[i].position - ofs) * scale;
        transform.translation = Vec3::new(v.x as f32, v.y as f32, v.z as f32);
        //transform.scale = scale;
        *star = stars[i];
    }
}

fn camera_orbit(
    time: Res<Time>,
    mut camera_transforms: Query<&mut Transform, With<Camera>>,
    mut angle: ResMut<CameraAngle>,
) {
    if let Ok(mut transform) = camera_transforms.single_mut() {
        let time_delta = time.delta().as_secs_f32();
        let len = 2500.0;
        let x = angle.0.cos() * len;
        let y = angle.0.sin() * len;
        *transform.translation = Vec3::new(x, y, len).into();
        transform.look_at(Vec3::ZERO, Vec3::Z);
        angle.0 += time_delta * camera_speed;
        if angle.0 > 2.0 * PI {
            angle.0 -= 2.0 * PI;
        }
    }
}
