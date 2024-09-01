//! The system implementation used by default when the `std` feature is enabled.

use crate::Transport;

/// System implementation using the standard library.
#[derive(Debug)]
pub struct StdSystem<T> {
	_transport: T,
}

impl<T> crate::System for StdSystem<T>
where
	T: Transport,
{
	type Transport = T;
}
