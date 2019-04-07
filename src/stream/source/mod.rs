pub mod iter;

use std::iter as std_iter;

pub use iter::Iter;

/// A `Source` is a convention for `Publish`ers that are not
/// `Subscriber`s, i.e. they have no input but one output.
///
/// This provides convenience functions for creating common
/// sources.
pub struct Sources;

impl Sources {
    pub fn iterator<A, I: Iterator<Item = A>>(iterator: I) -> iter::Iter<A, I>
    where
        A: 'static + Send,
        I: 'static + Send,
    {
        iter::Iter::new(iterator)
    }

    pub fn empty<A>() -> iter::Iter<A, std_iter::Empty<A>>
    where
        A: Send + Clone,
    {
        iter::Iter::new(std_iter::empty())
    }

    pub fn repeat<A>(element: A) -> iter::Iter<A, std_iter::Repeat<A>>
    where
        A: Send + Clone,
    {
        iter::Iter::new(std_iter::repeat(element))
    }

    pub fn single<A>(element: A) -> iter::Iter<A, std_iter::Once<A>>
    where
        A: Send,
    {
        iter::Iter::new(std_iter::once(element))
    }
}
