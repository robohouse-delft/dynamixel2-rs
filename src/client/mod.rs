#[path = "."]
pub(crate) mod asynch {
	use crate::asynch::SerialPort;
	use crate::bus::asynch::Bus;
	use bisync::asynchronous::*;
	#[cfg(feature = "serial2-tokio")]
	use serial2_tokio::SerialPort as Serial2Port;

	mod client;
	pub(crate) use client::Client;
}
#[path = "."]
pub(crate) mod sync {
	use crate::bus::sync::Bus;
	use crate::SerialPort;
	use bisync::synchronous::*;
	#[cfg(feature = "serial2")]
	use serial2::SerialPort as Serial2Port;

	mod client;
	pub(crate) use client::Client;
}
