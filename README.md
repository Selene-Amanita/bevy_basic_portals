# Portals for Bevy

[![crates.io](https://img.shields.io/crates/v/bevy_basic_portals)](https://crates.io/crates/bevy_basic_portals)
[![docs.rs](https://img.shields.io/docsrs/bevy_basic_portals)](https://docs.rs/bevy_basic_portals/latest/bevy_basic_portals/)
[![NVPLv7+](https://img.shields.io/badge/license-NPLv7+-blue.svg)](https://git.pixie.town/thufie/npl-builder/src/branch/main/nvpl.md)
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/quick-start/plugin-development/#main-branch-tracking)

Bevy Basic Portals is a Bevy game engine plugin aimed to create portals.

Those portals are (for now) purely visual and can be used to make mirrors, indoor renderings, crystal balls, and more!

![Portal Cube example](https://github.com/Selene-Amanita/bevy_basic_portal/assets/134181069/9864c08c-7826-4b4a-bea1-082c4434fd74) ![Moving portals and destination example](https://github.com/Selene-Amanita/bevy_basic_portal/assets/134181069/14474b43-c5df-41ca-9d60-cb604fb4997b) ![Mirror example](https://github.com/Selene-Amanita/bevy_basic_portals/assets/134181069/b34e34b7-08ca-483c-8ff7-d31869e1b22d)

## Basic Usage
This example illustrates how to create a simple portal, it uses a single sphere that will be displayed two times on screen thanks to the portal:
```rust
use bevy::prelude::*;
use bevy_basic_portals::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PortalsPlugin::MINIMAL))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-20.0, 0., 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let portal_mesh = meshes.add(Mesh::from(Rectangle::new(10., 10.)));
    commands.spawn(CreatePortalBundle {
        mesh: portal_mesh,
        // This component will be deleted and things that are needed to create the portal will be created
        create_portal: CreatePortal {
            destination: AsPortalDestination::Create(CreatePortalDestination {
                transform: Transform::from_xyz(20., 0., 0.),
                ..default()
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

    let sphere_mesh = meshes.add(Mesh::from(Sphere::new(2.).mesh().uv(32, 18)));
    commands.spawn(PbrBundle {
        mesh: sphere_mesh,
        transform: Transform::from_xyz(20., 0., -5.),
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
- portals created by this crate are uni-directionnal, you can only look from one space to the other,
if you want a bidirectional portal you can crate two portals manually
- this crate doesn't handle "portal recursion", as in viewing a portal through another portal
- portals created by this crate have no visible borders (not counting aliasing artifacts), you can "see" them with `DebugPortal`
- this crate doesn't handle moving stuff through the portal, it is only visual, more like a crystal ball
- this crate doesn't handle raycasting through the portal, it has to be done manually
- this crate doesn't handle changing the portal's or the destination's scale
- this crate doesn't handle changing camera settings after creation

## Bevy versions
| Bevy version | Bevy Basic Portals recommended version |
|--------------|----------------------------------------|
| 0.14.*       | 0.6.0                                  |
| 0.13.*       | 0.5.0                                  |
| 0.12.*       | 0.4.0                                  |
| 0.11.1       | 0.3.0                                  |
| 0.10.*       | 0.2.1                                  |

## License

Bevy Basic Portals is non-violent software: you can use, redistribute, and/or modify it under the terms of the `NVPLv7+` as found in the `LICENSE` file in the source code root directory or at [the CNPL repository](https://git.pixie.town/thufie/npl-builder/src/branch/main/nvpl.md).

Bevy Basic Portals comes with ABSOLUTELY NO WARRANTY, to the extent permitted by applicable law.

See [the CNPL website](https://thufie.lain.haus/NPL.html) for more information.