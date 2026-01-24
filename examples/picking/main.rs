//! This example illustrates how to create a simple portal,
//! it uses a single sphere that will be displayed two times on screen thanks to the portal

use bevy::prelude::*;
use bevy_basic_portals::*;
use bevy_color::palettes::css::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PortalsPlugin::MINIMAL, MeshPickingPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-20.0, 0., 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let portal_mesh = meshes.add(Mesh::from(Rectangle::new(10., 10.)));
    commands.spawn((
        CreatePortal {
            destination: PortalDestinationSource::Create(CreatePortalDestination {
                transform: Transform::from_xyz(20., 0., 0.),
                ..default()
            }),
            debug: Some(DebugPortal {
                show_portal_texture: DebugPortalTextureView::None,
                ..default()
            }),
            ..default()
        },
        Mesh3d(portal_mesh),
    ));

    let sphere_mesh = meshes.add(Mesh::from(Sphere::new(2.).mesh().uv(32, 18)));
    commands
        .spawn((
            Mesh3d(sphere_mesh),
            MeshMaterial3d(materials.add(Color::Srgba(RED))),
            Transform::from_xyz(20., 0., -5.),
        ))
        .observe(on_event_change_color::<Over, true>)
        .observe(on_event_change_color::<Out, false>);

    commands.insert_resource(GlobalAmbientLight {
        brightness: 500.,
        ..default()
    });
}

fn on_event_change_color<E: std::fmt::Debug + Clone + Reflect, const MAKE_GREEN: bool>(
    trigger: On<Pointer<E>>,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_handle = material_query.get(trigger.event().entity).unwrap();
    let material = materials.get_mut(material_handle).unwrap();
    material.base_color = Color::Srgba(if MAKE_GREEN { GREEN } else { RED });
}
