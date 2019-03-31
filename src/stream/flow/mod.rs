pub mod detached;
pub mod filter;
pub mod filter_map;
pub mod identity;
pub mod map;
pub mod scan;

pub use detached::Detached;
pub use filter::Filter;
pub use filter_map::FilterMap;
//pub use identity::Identity;
pub use map::Map;
//pub use scan::Scan;

/// A `Flow` is a convention for `Subscriber`s that are also
/// `Publish`ers. This type is currently not used for much,
/// as flows currently appear to be better expressed as
/// simple functions given the convenience of `impl` traits.
pub struct Flow;
