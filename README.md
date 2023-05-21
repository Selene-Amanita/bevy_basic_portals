# Portals for Bevy

[![MIT/Apache 2.0](https://img.shields.io/badge/license-EUPL-blue.svg)](https://commission.europa.eu/content/european-union-public-licence_en)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

Bevy Simple Portals is a Bevy game engine plugin aimed to create portals.

Those portals are (for now) purely visual and can be used to make mirrors, indoor renderings, crystal balls, and more!

## Basic Usage
This example illustrates how to create a simple portal, it uses a single sphere that will be displayed two times on screen thanks to the portal:
```rust
use bevy::prelude::*;
use bevy_basic_portals::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PortalsPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-20.0, 0., 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let portal_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(10., 10.))));
    commands.spawn(CreatePortalBundle {
        mesh: portal_mesh,
        // This component will be deleted and things that are needed to create the portal will be created
        create_portal: CreatePortal {
            destination: AsPortalDestination::Create(CreatePortalDestination {
                transform: Transform::from_xyz(20., 0., 0.),
            }),
            // Uncomment this to see the portal
            /*debug: Some(DebugPortal {
                show_window: false,
                ..default()
            }),*/
            ..default()
        },
        ..default()
    });

    let sphere_mesh = meshes.add(Mesh::from(shape::UVSphere{radius: 2., ..default()}));
    commands.spawn(PbrBundle {
        mesh: sphere_mesh,
        transform: Transform::from_xyz(20.,0.,-5.),
        ..default()
    });
}
```
More complex examples are available in the examples folder.

## Vocabulary
- A Portal is an entity used to visualise the effect
- A Main Camera is a camera used to visualize the effect
- A (portal) Destination is an entity representing the point in space where a portal is "looking"
- A Portal Camera is a camera being used to render the effect, its position to the destination is the same as the main camera's position to the portal

## Known limitations
(may be fixed in the future)
- this crate doesn't define a correct frustum, which can pose a problem if an object is between the portal camera and the portal destination
and also can reduce perfomance by rendering things that are not displayed
(see (https://tomhulton.blogspot.com/2015/08/portal-rendering-with-offscreen-render.html) and (https://www.youtube.com/watch?v=cWpFZbjtSQg))
- portals created by this crate are uni-directionnal, you can only look from one space to the other,
if you want a bidirectional portal you can crate two portals manually
- this crate doesn't handle "portal recursion", as in viewing a portal through another portal
- portals created by this crate have no visible borders (not counting aliasing artifacts), you can 'see" them with DebugPortal
- this crate doesn't automatically handle mirrors,
they have to be done manually by rotating the portal's transform 180° to use it as the destination's transform
- this crate doesn't handle removing portals (it can be done manually to some extent)
- this crate doesn't handle moving stuff through the portal, it is only visual, more like a crystal ball
- this crate doesn't handle raycasting through the portal, it has to be done manually
- this crate doesn't handle resizing window/viewport of the main camera
- this crate doesn't handle changing the portal's or the destination's scale
- this crate doesn't work if cameras, portals, or destinations are inside hierarchies (it uses Transform instead of GlobalTransform)