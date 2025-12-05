#[path = "."]
mod asynch {
	use bisync::asynchronous::*;
	pub(super) mod serial_port;
}
#[path = "."]
mod sync {
	use bisync::synchronous::*;
	pub(super) mod serial_port;
}

pub use asynch::serial_port::SerialPort as AsyncSerialPort;
pub use sync::serial_port::SerialPort;

#[cfg(feature = "serial2")]
mod serial2;
#[cfg(feature = "serial2")]
mod serial2_tokio;
