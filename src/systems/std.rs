use crate::systems::serial_port::SerialPort;
use crate::Transport;

#[derive(Debug)]
pub struct StdSystem<T = SerialPort> {
	_transport: T,
}

impl<T> crate::System for StdSystem<T>
where
	T: Transport,
{
	type Transport = T;
}
