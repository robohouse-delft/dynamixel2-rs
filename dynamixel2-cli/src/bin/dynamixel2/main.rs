use serial::SerialPort;
use std::path::Path;
use structopt::StructOpt;

mod logging;
mod options;

use options::{Command, MotorId, Options};

fn main() {
	let options = Options::from_args();
	logging::init(env!("CARGO_CRATE_NAME"), options.verbose);
	if let Err(()) = do_main(options) {
		std::process::exit(1);
	}
}

fn do_main(options: Options) -> Result<(), ()> {
	match &options.command {
		Command::Ping { motor_id } => {
			let mut bus = open_bus(&options)?;
			match motor_id {
				&MotorId::Id(motor_id) => {
					log::debug!("Sending ping command to motor {}", motor_id);
					let response = bus.ping(motor_id).map_err(|e| log::error!("Command failed: {}", e))?;
					log_ping_response(&response);
				},
				MotorId::Broadcast => {
					log::debug!("Scanning bus for connected motors");
					bus.scan_cb(|response| match response {
						Ok(response) => log_ping_response(&response),
						Err(e) => log::warn!("Communication error: {}", e),
					})
					.map_err(|e| log::error!("Command failed: {}", e))?;
				},
			}
		},
		Command::Reboot { motor_id } => {
			let mut bus = open_bus(&options)?;
			log::debug!("Sending reboot command with motor ID {}", motor_id.raw());
			bus.reboot(motor_id.raw()).map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok");
		},
		Command::Read8 { motor_id, address } => {
			let mut bus = open_bus(&options)?;
			log::debug!("Reading an 8-bit value from motor {} at address {}", motor_id.raw(), address);
			let value = bus
				.read_u8(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok: {} (0x{:02X})", value, value);
		},
		Command::Read16 { motor_id, address } => {
			let mut bus = open_bus(&options)?;
			log::debug!("Reading a 16-bit value from motor {} at address {}", motor_id.raw(), address);
			let value = bus
				.read_u16(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok: {} (0x{:04X})", value, value);
		},
		Command::Read32 { motor_id, address } => {
			let mut bus = open_bus(&options)?;
			log::debug!("Reading a 32-bit value from motor {} at address {}", motor_id.raw(), address);
			let value = bus
				.read_u32(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok: {} (0x{:04X} {:04X})", value, (value >> 16) & 0xFFFF, value & 0xFFFF);
		},
		Command::Write8 { motor_id, address, value } => {
			let mut bus = open_bus(&options)?;
			log::debug!(
				"Writing 8-bit value {} (0x{:02X}) to motor {} at address {}",
				value,
				value,
				motor_id.raw(),
				address
			);
			bus.write_u8(motor_id.raw(), *address, *value)
				.map_err(|e| log::error!("Write failed: {}", e))?;
			log::info!("Ok");
		},
		Command::Write16 { motor_id, address, value } => {
			let mut bus = open_bus(&options)?;
			log::debug!(
				"Writing 16-bit value {} (0x{:04X}) to motor {} at address {}",
				value,
				value,
				motor_id.raw(),
				address
			);
			bus.write_u16(motor_id.raw(), *address, *value)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok");
		},
		Command::Write32 { motor_id, address, value } => {
			let mut bus = open_bus(&options)?;
			log::debug!(
				"Writing 32-bit value {} (0x{:04X} {:04X}) to motor {} at address {}",
				value,
				value >> 16,
				value & 0xFFFF,
				motor_id.raw(),
				address
			);
			bus.write_u32(motor_id.raw(), *address, *value)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			log::info!("Ok");
		},
		Command::ShellCompletion { shell, output } => {
			write_shell_completion(*shell, output.as_deref())?;
		},
	}

	Ok(())
}

fn open_bus(options: &Options) -> Result<dynamixel2::Bus<serial::SystemPort, Vec<u8>, Vec<u8>>, ()> {
	let mut serial_port = serial::SystemPort::open(&options.serial_port)
		.map_err(|e| log::error!("Failed to open serial port: {}: {}", options.serial_port.display(), e))?;
	serial_port
		.configure(&serial::PortSettings {
			baud_rate: serial::BaudRate::from_speed(options.baud_rate),
			char_size: serial::Bits8,
			stop_bits: serial::Stop1,
			parity: serial::ParityNone,
			flow_control: serial::FlowNone,
		})
		.map_err(|e| log::error!("Failed to configure serial port: {}: {}", options.serial_port.display(), e))?;
	log::debug!(
		"Using serial port {} with baud rate {}",
		options.serial_port.display(),
		options.baud_rate
	);

	Ok(dynamixel2::Bus::new(serial_port, std::time::Duration::from_millis(50)))
}

fn log_ping_response(response: &dynamixel2::instructions::PingResponse) {
	log::info!("Motor ID: {}", response.motor_id);
	log::info!("Model: {}", response.model);
	log::info!("Firmware: {}", response.firmware);
}

fn write_shell_completion(shell: structopt::clap::Shell, path: Option<&Path>) -> Result<(), ()> {
	use std::io::Write;
	let mut buffer = Vec::with_capacity(4 * 1024);

	Options::clap().gen_completions_to(env!("CARGO_BIN_NAME"), shell, &mut buffer);
	if !buffer.ends_with(b"\n") {
		buffer.push(b'\n');
	}

	let path = path.unwrap_or_else(|| Path::new("-"));
	if path == Path::new("-") {
		log::debug!("Writing shell completion for {} to stdout", shell);
		let stdout = std::io::stdout();
		stdout
			.lock()
			.write_all(&buffer)
			.map_err(|e| log::error!("Failed to write to stdout: {}", e))?;
	} else {
		log::debug!("Writing shell completion for {} to {}", shell, path.display());
		let mut output = std::fs::File::create(path).map_err(|e| log::error!("Failed to create {}: {}", path.display(), e))?;
		output
			.write_all(&buffer)
			.map_err(|e| log::error!("Failed to write to {}: {}", path.display(), e))?;
	}

	Ok(())
}
