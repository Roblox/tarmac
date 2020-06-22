//! Packos is a small library for packing rectangles. It was built for
//! [Tarmac](https://github.com/Roblox/tarmac), a tool that manages assets for
//! Roblox projects, including packing images into spritesheets.
//!
//! Packos currently exposes a single packing implementation,
//! [`SimplePacker`][SimplePacker]. More algorithms can be added in the future
//! using the same basic types that Packos uses.
//!
//! ## Example
//! ```
//! use packos::{InputItem, SimplePacker};
//!
//! // First, transform the rectangles you want to pack into the Packos
//! // InputItem type.
//! let my_items = &[
//!     InputItem::new((128, 64)),
//!     InputItem::new((64, 64)),
//!     InputItem::new((1, 300)),
//! ];
//!
//! // Construct a packer and configure it with your constraints
//! let packer = SimplePacker::new().max_size((512, 512));
//!
//! // Compute a solution.
//! // SimplePacker::pack accepts anything that can turn into an iterator of
//! // InputItem or &InputItem.
//! let output = packer.pack(my_items);
//! ```
//!
//! [SimplePacker]: struct.SimplePacker.html

mod geometry;
mod id;
mod packer;
mod types;

pub use id::*;
pub use packer::*;
pub use types::*;
