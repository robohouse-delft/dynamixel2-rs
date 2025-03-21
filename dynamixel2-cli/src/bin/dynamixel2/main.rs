use std::path::Path;
use std::time::{Duration, Instant};

mod logging;
mod options;

use options::{Command, MotorId, Options};

fn main() {
	if let Err(()) = do_main(clap::Parser::parse()) {
		std::process::exit(1);
	}
}

fn do_main(options: Options) -> Result<(), ()> {
	logging::init(module_path!(), options.verbose as i8);
	match &options.command {
		Command::Ping { motor_id } => {
			let mut client = open_client(&options)?;
			match motor_id {
				&MotorId::Id(motor_id) => {
					log::debug!("Sending ping command to motor {}", motor_id);
					let start = Instant::now();
					let response = client.ping(motor_id).map_err(|e| log::error!("Command failed: {}", e))?;
					if response.alert {
						log::warn!("Alert bit set in response from motor!")
					}
					log::info!("{:?}: {:?}", start.elapsed(), response.data);
				},
				MotorId::Broadcast => {
					let start = Instant::now();
					let scan = client.scan().map_err(|e| log::error!("Command failed: {}", e))?;
					for s in scan {
						match s {
							Ok(r) => log_ping_response(&r, start.elapsed()),
							Err(e) => log::error!("Error pinging motor: {}", e),
						}
					}
				},
			}
		},
		Command::Reboot { motor_id } => {
			let mut client = open_client(&options)?;
			log::debug!("Sending reboot command with motor ID {}", motor_id.raw());
			let start = Instant::now();
			let response = client.reboot(motor_id.raw()).map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: Ok", start.elapsed());
		},
		Command::Read8 { motor_id, address } => {
			let mut client = open_client(&options)?;
			log::debug!("Reading an 8-bit value from motor {} at address {}", motor_id.raw(), address);
			let start = Instant::now();
			let response = client
				.read::<u8>(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: {:?} (0x{:02X})", start.elapsed(), response.data, response.data);
		},
		Command::Read16 { motor_id, address } => {
			let mut client = open_client(&options)?;
			log::debug!("Reading a 16-bit value from motor {} at address {}", motor_id.raw(), address);
			let start = Instant::now();
			let response = client
				.read::<u16>(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: {:?} (0x{:04X})", start.elapsed(), response.data, response.data);
		},
		Command::Read32 { motor_id, address } => {
			let mut client = open_client(&options)?;
			log::debug!("Reading a 32-bit value from motor {} at address {}", motor_id.raw(), address);
			let start = Instant::now();
			let response = client
				.read::<u32>(motor_id.assume_unicast()?, *address)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!(
				"{:?}: {:?} (0x{:04X} {:04X})",
				start.elapsed(),
				response.data,
				(response.data >> 16) & 0xFFFF,
				response.data & 0xFFFF
			);
		},
		Command::Read { motor_id, address, count } => {
			let mut client = open_client(&options)?;
			log::debug!("Reading a {} bytes from motor {} at address {}", count, motor_id.raw(), address);
			let start = Instant::now();
			let response = client
				.read_bytes::<&[u8]>(motor_id.assume_unicast()?, *address, *count)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: {:?}", start.elapsed(), response.data,);
		},
		Command::Write8 { motor_id, address, value } => {
			let mut client = open_client(&options)?;
			log::debug!(
				"Writing 8-bit value {} (0x{:02X}) to motor {} at address {}",
				value,
				value,
				motor_id.raw(),
				address
			);
			let start = Instant::now();
			let response = client
				.write(motor_id.raw(), *address, value)
				.map_err(|e| log::error!("Write failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: Ok", start.elapsed());
		},
		Command::Write16 { motor_id, address, value } => {
			let mut client = open_client(&options)?;
			log::debug!(
				"Writing 16-bit value {} (0x{:04X}) to motor {} at address {}",
				value,
				value,
				motor_id.raw(),
				address
			);
			let start = Instant::now();
			let response = client
				.write(motor_id.raw(), *address, value)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: Ok", start.elapsed());
		},
		Command::Write32 { motor_id, address, value } => {
			let mut client = open_client(&options)?;
			log::debug!(
				"Writing 32-bit value {} (0x{:04X} {:04X}) to motor {} at address {}",
				value,
				value >> 16,
				value & 0xFFFF,
				motor_id.raw(),
				address
			);
			let start = Instant::now();
			let response = client
				.write(motor_id.raw(), *address, value)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: Ok", start.elapsed());
		},
		Command::Write { motor_id, address, data } => {
			let mut client = open_client(&options)?;
			log::debug!("Writing {} bytes to motor {} at address {}", data.len(), motor_id.raw(), address);
			let start = Instant::now();
			let response = client
				.write_bytes(motor_id.raw(), *address, &data)
				.map_err(|e| log::error!("Command failed: {}", e))?;
			if response.alert {
				log::warn!("Alert bit set in response from motor!")
			}
			log::info!("{:?}: Ok", start.elapsed());
		},
		Command::ShellCompletion { shell, output } => {
			write_shell_completion(*shell, output.as_deref())?;
		},
	}

	Ok(())
}

fn open_client(options: &Options) -> Result<dynamixel2::Client, ()> {
	let client = dynamixel2::Client::open(&options.serial_port, options.baud_rate)
		.map_err(|e| log::error!("Failed to open serial port: {}: {}", options.serial_port.display(), e))?;
	log::debug!(
		"Using serial port {} with baud rate {}",
		options.serial_port.display(),
		options.baud_rate
	);
	Ok(client)
}

fn log_ping_response(response: &dynamixel2::Response<dynamixel2::instructions::Ping>, elapsed: Duration) {
	log::info!("Motor ID: {}", response.motor_id);
	log::info!(" ├─ Response time: {:?}", elapsed);
	if response.alert {
		log::info!(" ├─ Alert: true")
	}
	log::info!(" ├─ Model: {}", response.data.model);
	log::info!(" └─ Firmware: {}", response.data.firmware);
}

fn write_shell_completion(shell: clap_complete::Shell, path: Option<&Path>) -> Result<(), ()> {
	use clap::CommandFactory;
	use std::io::Write;

	let mut buffer = Vec::with_capacity(4 * 1024);

	let mut command = Options::command();
	clap_complete::generate(shell, &mut command, env!("CARGO_BIN_NAME"), &mut buffer);
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
