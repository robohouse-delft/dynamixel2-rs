// mod _logging;
use dynamixel2::{Bus, MotorError, ReadError, TransferError};
use std::path::PathBuf;

const PRESENT_POSITION: (u16, u16) = (132, 4);

fn main() {
	let serial_port: PathBuf = "/dev/ttyUSB0".into();
	// _logging::init(env!("CARGO_CRATE_NAME"), 3);
	let mut bus = Bus::open_with_buffers(
		&serial_port,
		4000000,
		std::time::Duration::from_millis(20),
		vec![0; 1024],
		vec![0; 1024],
	)
	.map_err(|e| println!("Failed to open serial port: {}: {}", serial_port.display(), e))
	.unwrap();

	let response: Result<u32, TransferError<u32>> = bus.read_u32(37, PRESENT_POSITION.0);
	let present_position = match response {
		Err(TransferError::ReadError(ReadError::MotorError(MotorError::HardwareError(data)))) => {
			println!("Motor has hardware Error");
			data
		},
		Err(e) => panic!("Error: {:?}", e),
		Ok(data) => data,
	};
	println!("Present Position: {}", present_position)
}
