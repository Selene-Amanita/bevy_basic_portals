//! Bevy Basic Portals is a Bevy game engine plugin aimed to create portals.
//!
//! Those portals are (for now) purely visual and can be used to make mirrors, indoor renderings, crystal balls, and more!
//!
//! ## Basic Usage
//! ```rust
#![doc = include_str!("../examples/basic/main.rs")]
//! ```
//! More complex examples are available in the [git project](https://github.com/Selene-Amanita/bevy_basic_portals).
//!
//! ## Vocabulary
//! - A Portal is an entity used to visualise the effect
//! - A Main Camera is a camera used to visualize the effect
//! - A (portal) Destination is an entity representing the point in space where a portal is "looking"
//! - A Portal Camera is a camera being used to render the effect, its position to the destination is the same as the main camera's position to the portal
//! - All of those are Portal Parts, their references are stored in a separate entity
//!
//! ## Known limitations
//! (may be fixed in the future)
//! - portals created by this crate are uni-directionnal, you can only look from one space to the other,
//! if you want a bidirectional portal you can crate two portals manually
//! - this crate doesn't handle "portal recursion", as in viewing a portal through another portal
//! - portals created by this crate have no visible borders (not counting aliasing artifacts), you can "see" them with [DebugPortal]
//! - this crate doesn't handle moving stuff through the portal, it is only visual, more like a crystal ball
//! - this crate doesn't handle raycasting through the portal, it has to be done manually
//! - this crate doesn't handle resizing window/viewport of the main camera
//! - this crate doesn't handle changing the portal's or the destination's scale
//! - this crate doesn't handle changing camera settings after creation

pub mod portals;
pub use portals::*;
#[cfg(feature = "picking_backend")]
pub mod picking;
#[doc(inline)]
pub use portals::{CreatePortal, PortalsPlugin};

//TODO examples:
// - should handle multiple main cameras
// - portal or destination in a hierarchy should work
// - spawning portals, despawning portal parts, removing necessary components
