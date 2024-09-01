use crate::Transport;

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
