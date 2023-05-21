//! Bevy Simple Portals is a Bevy game engine plugin aimed to create portals.
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
//!
//! ## Known limitations
//! (may be fixed in the future)
//! - this crate doesn't define a correct frustum, which can pose a problem if an object is between the portal camera and the portal destination
//! and also can reduce perfomance by rendering things that are not displayed
//! (see <https://tomhulton.blogspot.com/2015/08/portal-rendering-with-offscreen-render.html> and <https://www.youtube.com/watch?v=cWpFZbjtSQg>)
//! - portals created by this crate are uni-directionnal, you can only look from one space to the other,
//! if you want a bidirectional portal you can crate two portals manually
//! - this crate doesn't handle "portal recursion", as in viewing a portal through another portal
//! - portals created by this crate have no visible borders (not counting aliasing artifacts), you can 'see" them with [DebugPortal]
//! - this crate doesn't automatically handle mirrors,
//! they have to be done manually by rotating the portal's transform 180° to use it as the destination's transform
//! - this crate doesn't handle removing portals (it can be done manually to some extent)
//! - this crate doesn't handle moving stuff through the portal, it is only visual, more like a crystal ball
//! - this crate doesn't handle raycasting through the portal, it has to be done manually
//! - this crate doesn't handle resizing window/viewport of the main camera
//! - this crate doesn't handle changing the portal's or the destination's scale
//! - this crate doesn't work if cameras, portals, or destinations are inside hierarchies (it uses Transform instead of GlobalTransform)

pub mod portals;
pub use portals::*;
#[doc(inline)]
pub use portals::{PortalsPlugin, CreatePortalBundle};

//TODO examples:
// - should handle multiple main cameras