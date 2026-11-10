//! Day/night cycle, sky dome, stars, sun and moon.

use crate::*;

pub(crate) const GAME_DAY_SECONDS: f32 = 24.0 * 60.0 * 60.0;

pub(crate) const GAME_SECONDS_PER_REAL_SECOND: f32 = 60.0;

pub(crate) const DEFAULT_TIME_OF_DAY_SECONDS: f32 = 8.0 * 60.0 * 60.0;

pub(crate) const SKY_RADIUS: f32 = 180.0;

pub(crate) const CELESTIAL_RADIUS: f32 = 120.0;

pub(crate) const STAR_RADIUS: f32 = 145.0;

#[derive(Resource)]
pub(crate) struct DayNightCycle {
    pub(crate) time_of_day_seconds: f32,
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            time_of_day_seconds: DEFAULT_TIME_OF_DAY_SECONDS,
        }
    }
}

#[derive(Component)]
pub(crate) struct SkyDome;

#[derive(Component)]
pub(crate) struct SkyStars;

#[derive(Component)]
pub(crate) struct SunCube;

#[derive(Component)]
pub(crate) struct MoonCube;

#[derive(Component)]
pub(crate) struct SunLighting;

#[derive(Component)]
pub(crate) struct MoonLighting;

pub(crate) fn advance_day_night(time: Res<Time>, mut cycle: ResMut<DayNightCycle>) {
    cycle.time_of_day_seconds = advance_time_of_day(
        cycle.time_of_day_seconds,
        time.delta_secs(),
        GAME_SECONDS_PER_REAL_SECOND,
    );
}

pub(crate) fn update_day_night_lighting(
    cycle: Res<DayNightCycle>,
    mut sun: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<SunLighting>, Without<MoonLighting>),
    >,
    mut moon: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<MoonLighting>, Without<SunLighting>),
    >,
    mut ambient: ResMut<AmbientLight>,
) {
    let (elevation, daylight) = sun_position_and_daylight(cycle.time_of_day_seconds);
    let sunlight = if elevation > 0.0 {
        smoothstep(0.0, 0.15, elevation)
    } else {
        0.0
    };
    let sun_direction = sun_direction(cycle.time_of_day_seconds);

    if let Ok((mut transform, mut light)) = sun.get_single_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction);
        light.illuminance = sunlight * 3_000.0;
        light.color = Color::srgb(1.0, 0.62 + daylight * 0.38, 0.42 + daylight * 0.58);
    }

    if let Ok((mut transform, mut light)) = moon.get_single_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, sun_direction);
        light.illuminance = (1.0 - sunlight) * 160.0;
        light.color = Color::srgb(0.48, 0.62, 1.0);
    }

    ambient.color = Color::srgb(
        0.62 + sunlight * 0.16,
        0.65 + sunlight * 0.15,
        0.70 + sunlight * 0.12,
    );
    ambient.brightness = 350.0 + sunlight * 700.0;
}

pub(crate) fn update_sky(
    cycle: Res<DayNightCycle>,
    player: Query<&Transform, With<Player>>,
    mut sky: Query<(&mut Transform, &Mesh3d), (With<SkyDome>, Without<Player>)>,
    mut stars: Query<
        (&mut Transform, &MeshMaterial3d<StandardMaterial>),
        (With<SkyStars>, Without<Player>, Without<SkyDome>),
    >,
    mut sun: Query<
        (&mut Transform, &mut Visibility),
        (
            With<SunCube>,
            Without<Player>,
            Without<SkyDome>,
            Without<SkyStars>,
        ),
    >,
    mut moon: Query<
        (&mut Transform, &mut Visibility),
        (
            With<MoonCube>,
            Without<Player>,
            Without<SkyDome>,
            Without<SkyStars>,
            Without<SunCube>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player.get_single() else {
        return;
    };
    let center = player_transform.translation;
    let time = cycle.time_of_day_seconds;
    let (elevation, daylight) = sun_position_and_daylight(time);
    let direction = sun_direction(time);
    let cycle_time = time.rem_euclid(GAME_DAY_SECONDS);

    if let Ok((mut transform, mesh_handle)) = sky.get_single_mut() {
        transform.translation = center;
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, sky_vertex_colors(time));
        } else {
            error!("Missing sky dome mesh asset");
        }
    }

    if let Ok((mut transform, material_handle)) = stars.get_single_mut() {
        transform.translation = center;
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let opacity = (1.0 - smoothstep(0.08, 0.42, daylight)) * 0.95;
            material.base_color = Color::srgba(0.78, 0.86, 1.0, opacity);
        } else {
            error!("Missing star material asset");
        }
    }

    if let Ok((mut transform, mut visibility)) = sun.get_single_mut() {
        transform.translation = center + direction * CELESTIAL_RADIUS;
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            cycle_time * 0.0007,
            cycle_time * 0.0011,
            cycle_time * 0.0004,
        );
        *visibility = if elevation > -0.035 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut transform, mut visibility)) = moon.get_single_mut() {
        transform.translation = center - direction * CELESTIAL_RADIUS;
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            -cycle_time * 0.0003,
            cycle_time * 0.0005,
            cycle_time * 0.0002,
        );
        *visibility = if elevation < 0.035 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub(crate) fn advance_time_of_day(
    current: f32,
    delta_seconds: f32,
    game_seconds_per_real_second: f32,
) -> f32 {
    (current + delta_seconds.max(0.0) * game_seconds_per_real_second).rem_euclid(GAME_DAY_SECONDS)
}

pub(crate) fn sun_position_and_daylight(time_of_day_seconds: f32) -> (f32, f32) {
    let phase = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / GAME_DAY_SECONDS)
        * std::f32::consts::TAU;
    let elevation = (phase - std::f32::consts::FRAC_PI_2).sin();
    let daylight = smoothstep(-0.12, 0.2, elevation);
    (elevation, daylight)
}

pub(crate) fn sun_direction(time_of_day_seconds: f32) -> Vec3 {
    let (elevation, _) = sun_position_and_daylight(time_of_day_seconds);
    let phase = (time_of_day_seconds.rem_euclid(GAME_DAY_SECONDS) / GAME_DAY_SECONDS)
        * std::f32::consts::TAU;
    let horizontal = (1.0 - elevation * elevation).max(0.0).sqrt();
    Vec3::new(
        phase.cos() * horizontal,
        elevation,
        phase.sin() * horizontal,
    )
    .normalize_or_zero()
}

pub(crate) fn create_sky_dome_mesh() -> Mesh {
    const LATITUDE_SEGMENTS: usize = 32;
    const LONGITUDE_SEGMENTS: usize = 64;

    let mut positions = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1));
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);

    for latitude in 0..=LATITUDE_SEGMENTS {
        let vertical = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * latitude as f32 / LATITUDE_SEGMENTS as f32;
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let angle = std::f32::consts::TAU * longitude as f32 / LONGITUDE_SEGMENTS as f32;
            let direction = Vec3::new(
                vertical.cos() * angle.cos(),
                vertical.sin(),
                vertical.cos() * angle.sin(),
            );
            positions.push((direction * SKY_RADIUS).to_array());
            normals.push(direction.to_array());
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let top_left = (latitude * (LONGITUDE_SEGMENTS + 1) + longitude) as u32;
            let top_right = top_left + 1;
            let bottom_left = top_left + LONGITUDE_SEGMENTS as u32 + 1;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        sky_vertex_colors(DEFAULT_TIME_OF_DAY_SECONDS),
    );
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(crate) fn sky_vertex_colors(time_of_day_seconds: f32) -> Vec<[f32; 4]> {
    const LATITUDE_SEGMENTS: usize = 32;
    const LONGITUDE_SEGMENTS: usize = 64;
    let (elevation, daylight) = sun_position_and_daylight(time_of_day_seconds);
    let twilight = (1.0 - smoothstep(0.02, 0.26, elevation.abs())) * (1.0 - daylight * 0.7);
    let mut colors = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1));

    for latitude in 0..=LATITUDE_SEGMENTS {
        let vertical = -1.0 + 2.0 * latitude as f32 / LATITUDE_SEGMENTS as f32;
        let zenith_blend = smoothstep(-0.15, 0.9, vertical);
        for _ in 0..=LONGITUDE_SEGMENTS {
            let day_horizon = Vec3::new(0.58, 0.79, 0.98);
            let day_zenith = Vec3::new(0.12, 0.39, 0.82);
            let night_horizon = Vec3::new(0.025, 0.045, 0.13);
            let night_zenith = Vec3::new(0.006, 0.012, 0.045);
            let horizon = night_horizon.lerp(day_horizon, daylight);
            let zenith = night_zenith.lerp(day_zenith, daylight);
            let base = horizon.lerp(zenith, zenith_blend);
            let warm = Vec3::new(0.98, 0.31, 0.16).lerp(Vec3::new(0.52, 0.20, 0.32), zenith_blend);
            let twilight_amount = twilight * (1.0 - zenith_blend * 0.65);
            let color = base.lerp(warm, twilight_amount);
            colors.push([color.x, color.y, color.z, 1.0]);
        }
    }
    colors
}

pub(crate) fn create_stars_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut state = 0x9E37_79B9_u32;

    for _ in 0..180 {
        let azimuth = random_unit(&mut state) * std::f32::consts::TAU;
        let vertical = 0.08 + random_unit(&mut state) * 0.88;
        let horizontal = (1.0 - vertical * vertical).sqrt();
        let center = Vec3::new(
            horizontal * azimuth.cos(),
            vertical,
            horizontal * azimuth.sin(),
        ) * STAR_RADIUS;
        let half_size = 0.12 + random_unit(&mut state) * 0.18;
        append_cube(
            &mut positions,
            &mut normals,
            &mut indices,
            center,
            half_size,
        );
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(crate) fn append_cube(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    half_size: f32,
) {
    let h = half_size;
    let faces = [
        (
            [
                Vec3::new(-h, -h, h),
                Vec3::new(h, -h, h),
                Vec3::new(h, h, h),
                Vec3::new(-h, h, h),
            ],
            Vec3::Z,
        ),
        (
            [
                Vec3::new(h, -h, -h),
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, h, -h),
                Vec3::new(h, h, -h),
            ],
            Vec3::NEG_Z,
        ),
        (
            [
                Vec3::new(h, -h, h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, h, -h),
                Vec3::new(h, h, h),
            ],
            Vec3::X,
        ),
        (
            [
                Vec3::new(-h, -h, -h),
                Vec3::new(-h, -h, h),
                Vec3::new(-h, h, h),
                Vec3::new(-h, h, -h),
            ],
            Vec3::NEG_X,
        ),
        (
            [
                Vec3::new(-h, h, h),
                Vec3::new(h, h, h),
                Vec3::new(h, h, -h),
                Vec3::new(-h, h, -h),
            ],
            Vec3::Y,
        ),
        (
            [
                Vec3::new(-h, -h, -h),
                Vec3::new(h, -h, -h),
                Vec3::new(h, -h, h),
                Vec3::new(-h, -h, h),
            ],
            Vec3::NEG_Y,
        ),
    ];

    for (corners, normal) in faces {
        let base = positions.len() as u32;
        for corner in corners {
            positions.push((center + corner).to_array());
            normals.push(normal.to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

pub(crate) fn random_unit(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}
