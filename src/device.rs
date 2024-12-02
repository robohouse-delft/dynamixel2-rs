use crate::bus::endian::read_u16_le;
use crate::instructions::instruction_id;
use crate::bus::{Bus, InstructionPacket};
use crate::{InvalidParameterCount, ReadError, SerialPort, WriteError};
use core::time::Duration;

/// Dynamixel [`Device`] for implementing the device side of the DYNAMIXEL Protocol 2.0.
pub struct Device<ReadBuffer, WriteBuffer, T: SerialPort> {
	bus: Bus<ReadBuffer, WriteBuffer, T>,
}

impl<ReadBuffer, WriteBuffer, T> core::fmt::Debug for Device<ReadBuffer, WriteBuffer, T>
where
	T: SerialPort + core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Device")
			.field("serial_port", &self.bus.serial_port)
			.field("baud_rate", &self.bus.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
impl Device<Vec<u8>, Vec<u8>, serial2::SerialPort> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { bus })
	}

	/// Create a new device for an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(serial_port: serial2::SerialPort) -> std::io::Result<Self> {
		let bus = Bus::with_buffers(serial_port, vec![0; 128], vec![0; 128])?;
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2")]
impl<ReadBuffer, WriteBuffer> Device<ReadBuffer, WriteBuffer, serial2::SerialPort>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<std::path::Path>,
		baud_rate: u32,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(port, read_buffer, write_buffer, baud_rate);
		Ok(Self { bus })
	}
}
impl<ReadBuffer, WriteBuffer, T> Device<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Create a new device using pre-allocated buffers.
	pub fn with_buffers(
		serial_port: T,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> Result<Self, T::Error> {
		let bus = Bus::with_buffers(serial_port, read_buffer, write_buffer)?;
		Ok(Device { bus })
	}

	/// Get a reference to the underlying [`SerialPort`].
	///
	/// Note that performing any read or write with the [`SerialPort`] bypasses the read/write buffer of the device,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the device manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &T {
		&self.bus.serial_port
	}

	/// Consume this device object to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the device object.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> T {
		self.bus.serial_port
	}

	/// Get the baud rate of the device.
	pub fn baud_rate(&self) -> u32 {
		self.bus.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), T::Error> {
		self.bus.set_baud_rate(baud_rate)?;
		Ok(())
	}

	/// Read a single [`Instruction`] with borrowed data
	///
	/// Use [`Device::read_owned`] to received owned data
	pub fn read(&mut self, timeout: Duration) -> Result<Instruction<&[u8]>, ReadError<T::Error>> {
		let packet = self.read_raw_instruction_timeout(timeout)?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Read a single [`Instruction`] with borrowed data
	#[cfg(any(feature = "alloc", feature = "std"))]
	pub fn read_owned(&mut self, timeout: Duration) -> Result<Instruction<Vec<u8>>, ReadError<T::Error>> {
		let packet = self.read_raw_instruction_timeout(timeout)?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Write a status message to the device.
	pub fn write_status<F>(
		&mut self,
		packet_id: u8,
		error: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
	{
		self.bus
			.write_status(packet_id, error, parameter_count, encode_parameters)
	}

	/// Write an empty status message with an error code.
	pub fn write_status_error(&mut self, packet_id: u8, error: u8) -> Result<(), WriteError<T::Error>> {
		self.write_status(packet_id, error, 0, |_| {})
	}

	/// Write an empty status message.
	pub fn write_status_ok(&mut self, packet_id: u8) -> Result<(), WriteError<T::Error>> {
		self.write_status(packet_id, 0, 0, |_| {})
	}

	/// Read a single [`InstructionPacket`].
	pub fn read_raw_instruction_timeout(&mut self, timeout: Duration) -> Result<crate::bus::InstructionPacket<'_>, ReadError<T::Error>> {
		let deadline = T::make_deadline(self.serial_port(), timeout);
		loop {
			// SAFETY: This is a workaround for a limitation in the borrow checker.
			// Even though `packet` is dropped inside the loop body, it has lifetime 'a, which outlives the current function.
			// So each loop iteration tries to borrow the same field mutably, with the a lifetime that outlives the current function.
			// Borrow checker says no.
			let bus: &mut Bus<ReadBuffer, WriteBuffer, T> = unsafe { &mut * (&mut self.bus as *mut _) };
			let packet = bus.read_packet_deadline(deadline)?;
			if let Some(instruction) = packet.as_instruction() {
				return Ok(instruction)
			}
		}
	}
}

/// The options for the [Factory Reset](https://emanual.robotis.com/docs/en/dxl/protocol2/#factory-reset-0x06) instruction.
#[derive(Debug)]
pub enum FactoryReset {
	/// Reset all values to their factory defaults.
	All,
	/// Resets all values, except the ID.
	ExceptId,
	/// Resets all values, except the ID and baud rate.
	ExceptIdBaudRate,
	/// Reserved for future use.
	Unknown(u8),
}

/// The options for the [Clear](https://emanual.robotis.com/docs/en/dxl/protocol2/#clear-0x10) instruction.
#[derive(Debug)]
pub enum Clear {
	/// Reset the Present Position value to an absolute value within one rotation (0-4095).
	MultiTurns,
	/// Clear errors that occurred in DYNAMIXEL.
	/// If an error cannot be cleared or the conditions for clearance are not met, the error remains uncleared, and Result Fail (0x01) is displayed in the Error field of the Status Packet.
	/// Support only DYNAMIXEL Y series.
	Errors,
	/// Reserved for future use.
	Reserved(u8),
}

/// [`InstructionPacket`] can be converted into an [`Instruction`] with borrowed or owned data.
/// It contains the ID and parameters.
/// The owned data variant requires the `alloc` feature.
///
#[derive(Debug)]
pub struct Instruction<T> {
	/// The ID of the packet
	pub id: u8,
	/// The instruction as parsed into the [`Instructions`] enum
	pub instruction: Instructions<T>,
}

/// Instructions as defined in the [Dynamixel Protocol 2.0](https://emanual.robotis.com/docs/en/dxl/protocol2/#instruction-details).
///
/// The parameters are stored as a `&[u8]` slice or a `Vec<u8>`.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Instructions<T> {
	Ping,
	Read { address: u16, length: u16 },
	Write { address: u16, parameters: T },
	RegWrite { address: u16, parameters: T },
	Action,
	FactoryReset(FactoryReset),
	Reboot,
	Clear(Clear),
	SyncRead { address: u16, length: u16, ids: T },
	SyncWrite { address: u16, length: u16, parameters: T },
	BulkRead { parameters: T },
	BulkWrite { parameters: T },
	Unknown { instruction: u8, parameters: T },
}

impl<'a> TryFrom<InstructionPacket<'a>> for Instruction<&'a [u8]> {
	type Error = InvalidParameterCount;

	fn try_from(packet: InstructionPacket<'a>) -> Result<Self, Self::Error> {
		let id = packet.packet_id();
		let parameters: &'a [u8] = packet.parameters();
		let instruction = match packet.instruction_id() {
			instruction_id::PING => Instructions::Ping,
			instruction_id::READ => {
				InvalidParameterCount::check(parameters.len(), 4)?;
				Instructions::Read {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
				}
			},
			instruction_id::WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 2)?;
				Instructions::Write {
					address: read_u16_le(&parameters[..2]),
					parameters: &parameters[2..],
				}
			},
			instruction_id::REG_WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 2)?;
				Instructions::RegWrite {
					address: read_u16_le(&parameters[..2]),
					parameters: &parameters[2..],
				}
			},
			instruction_id::ACTION => Instructions::Action,
			instruction_id::FACTORY_RESET => {
				InvalidParameterCount::check(parameters.len(), 1)?;
				let reset_type = match parameters[0] {
					0xFF => FactoryReset::All,
					0x01 => FactoryReset::ExceptId,
					0x02 => FactoryReset::ExceptIdBaudRate,
					p => FactoryReset::Unknown(p),
				};
				Instructions::FactoryReset(reset_type)
			},
			instruction_id::REBOOT => Instructions::Reboot,
			instruction_id::CLEAR => {
				InvalidParameterCount::check_min(parameters.len(), 1)?;
				match parameters[0] {
					0x01 => Instructions::Clear(Clear::MultiTurns),
					0x02 => Instructions::Clear(Clear::Errors),
					p => Instructions::Clear(Clear::Reserved(p)),
				}
			},
			// todo: instruction_id::ControlTableBackup
			instruction_id::SYNC_READ => {
				InvalidParameterCount::check_min(parameters.len(), 4)?;
				Instructions::SyncRead {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
					ids: &parameters[4..],
				}
			},
			instruction_id::SYNC_WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 4)?;
				Instructions::SyncWrite {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
					parameters: &parameters[4..],
				}
			},
			instruction_id::BULK_READ => Instructions::BulkRead { parameters },
			instruction_id::BULK_WRITE => Instructions::BulkWrite { parameters },

			instruction => Instructions::Unknown { instruction, parameters },
		};

		Ok(Instruction { id, instruction })
	}
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> TryFrom<InstructionPacket<'a>> for Instruction<Vec<u8>> {
	type Error = InvalidParameterCount;

	fn try_from(packet: InstructionPacket<'a>) -> Result<Self, Self::Error> {
		let packet: Instruction<&[u8]> = packet.try_into()?;
		let Instruction { id, instruction } = packet;
		let instruction = match instruction {
			Instructions::Ping => Instructions::Ping,
			Instructions::Read { address, length } => Instructions::Read { address, length },
			Instructions::Write { address, parameters } => Instructions::Write {
				address,
				parameters: parameters.to_owned(),
			},
			Instructions::RegWrite { address, parameters } => Instructions::RegWrite {
				address,
				parameters: parameters.to_owned(),
			},
			Instructions::Action => Instructions::Action,
			Instructions::FactoryReset(f) => Instructions::FactoryReset(f),
			Instructions::Reboot => Instructions::Reboot,
			Instructions::Clear(c) => Instructions::Clear(c),
			Instructions::SyncRead { address, length, ids } => Instructions::SyncRead {
				address,
				length,
				ids: ids.to_owned(),
			},
			Instructions::SyncWrite {
				address,
				length,
				parameters,
			} => Instructions::SyncWrite {
				address,
				length,
				parameters: parameters.to_owned(),
			},
			Instructions::BulkRead { parameters } => Instructions::BulkRead {
				parameters: parameters.to_owned(),
			},
			Instructions::BulkWrite { parameters } => Instructions::BulkRead {
				parameters: parameters.to_owned(),
			},
			Instructions::Unknown { instruction, parameters } => Instructions::Unknown {
				instruction,
				parameters: parameters.to_owned(),
			},
		};
		Ok(Instruction { id, instruction })
	}
}
