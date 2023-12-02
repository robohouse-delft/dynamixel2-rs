use std::path::PathBuf;

/// Communicate with Dynamixel protocol 2.0 motors.
///
/// Most commands that take a motor ID can also take the special value "broadcast".
/// The only exceptions are the read commands, which can not be broadcasted.
#[derive(clap::Parser)]
pub struct Options {
	/// Print more verbose messages. Can be used multiple times.
	#[clap(long, short)]
	#[clap(global = true)]
	#[clap(action = clap::ArgAction::Count)]
	pub verbose: u8,

	/// The serial port to use.
	#[clap(long, short)]
	#[clap(global = true)]
	#[cfg_attr(target_os = "windows", clap(default_value = "COM1"))]
	#[cfg_attr(not(target_os = "windows"), clap(default_value = "/dev/ttyUSB0"))]
	pub serial_port: PathBuf,

	/// The baud rate for the serial port.
	#[clap(long, short)]
	#[clap(global = true)]
	#[clap(default_value = "9600")]
	pub baud_rate: u32,

	#[clap(subcommand)]
	pub command: Command,
}

#[derive(clap::Parser)]
pub enum Command {
	/// Ping a motor or scan the entire bus.
	Ping {
		/// The motor to ping.
		///
		/// You may specify the broadcast address to scan the bus for connected motors.
		#[clap(value_name = "MOTOR_ID|broadcast")]
		motor_id: MotorId,
	},

	/// Reboot a motor.
	Reboot {
		/// The motor to reboot.
		///
		/// You may specify the broadcast address to reboot all connected motors.
		#[clap(value_name = "MOTOR_ID|broadcast")]
		motor_id: MotorId,
	},

	/// Read an 8-bit value from a motor.
	Read8 {
		/// The motor to read from (no broadcast ID allowed).
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to read from.
		#[clap(value_name = "ADDRESS")]
		address: u16,
	},

	/// Read a 16-bit value from a motor.
	Read16 {
		/// The motor to read from (no broadcast ID allowed).
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to read from.
		#[clap(value_name = "ADDRESS")]
		address: u16,
	},

	/// Read a 32-bit value from a motor.
	Read32 {
		/// The motor to read from (no broadcast ID allowed).
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to read from.
		#[clap(value_name = "ADDRESS")]
		address: u16,
	},

	/// Write an 8-bit value to a motor.
	Write8 {
		/// The motor to write to.
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to write to.
		#[clap(value_name = "ADDRESS")]
		address: u16,

		/// The value to write.
		#[clap(value_name = "VALUE")]
		value: u8,
	},

	/// Write a 16-bit value to a motor.
	Write16 {
		/// The motor to write to.
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to write to.
		#[clap(value_name = "ADDRESS")]
		address: u16,

		/// The value to write.
		#[clap(value_name = "VALUE")]
		value: u16,
	},

	/// Write a 32-bit value to a motor.
	Write32 {
		/// The motor to write to.
		#[clap(value_name = "MOTOR_ID")]
		motor_id: MotorId,

		/// The address to write to.
		#[clap(value_name = "ADDRESS")]
		address: u16,

		/// The value to write.
		#[clap(value_name = "VALUE")]
		value: u32,
	},

	/// Write shell completions to standard output or a file.
	ShellCompletion {
		/// The shell for which to generate completions.
		#[clap(long)]
		shell: clap_complete::Shell,

		/// The file to write the generated completion file to.
		#[clap(long, short)]
		output: Option<PathBuf>,
	},
}
#[derive(Copy, Clone)]
pub enum MotorId {
	Id(u8),
	Broadcast,
}

impl MotorId {
	pub fn raw(self) -> u8 {
		match self {
			Self::Id(raw) => raw,
			Self::Broadcast => dynamixel2::instructions::packet_id::BROADCAST,
		}
	}

	pub fn assume_unicast(self) -> Result<u8, ()> {
		match self {
			Self::Id(raw) => Ok(raw),
			Self::Broadcast => {
				log::error!("Invalid motor ID: this command can not be broadcasted.");
				Err(())
			},
		}
	}
}

impl std::str::FromStr for MotorId {
	type Err = &'static str;

	fn from_str(data: &str) -> Result<Self, Self::Err> {
		if data.eq_ignore_ascii_case("broadcast") {
			Ok(Self::Broadcast)
		} else if let Ok(id) = data.parse() {
			if id == dynamixel2::instructions::packet_id::BROADCAST {
				Ok(Self::Broadcast)
			} else {
				Ok(Self::Id(id))
			}
		} else {
			Err("invalid motor ID: expected a number in the range 0..255 or the special value \"broadcast\"")
		}
	}
}
