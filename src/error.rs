use crate::{endian, instructions::packet_id::BROADCAST, Response};

/// An error that can occur during a read/write transfer.
#[derive(Debug)]
pub enum TransferError<T> {
	WriteError(WriteError),
	ReadError(ReadError<T>),
}

/// An error that can occur during a write transfer.
#[derive(Debug)]
pub enum WriteError {
	DiscardBuffer(std::io::Error),
	Write(std::io::Error),
}

/// An error that can occur during a read transfer.
#[derive(Debug)]
pub enum ReadError<T> {
	Io(std::io::Error),
	InvalidMessage(InvalidMessage),
	MotorError(MotorError<T>),
}

/// The received message is not valid.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InvalidMessage {
	InvalidHeaderPrefix(InvalidHeaderPrefix),
	InvalidChecksum(InvalidChecksum),
	InvalidPacketId(InvalidPacketId),
	InvalidInstruction(InvalidInstruction),
	InvalidParameterCount(InvalidParameterCount),
}

/// An error reported by the motor.
#[derive(Debug)]
pub enum MotorError<T> {
	/// The motor reported an error status.
	///
	/// The raw error status is available in the `raw` field.
	/// Refer to the online manual of your motor for the meaning of the error status.
	MotorError {
		raw: u8,
	},
	HardwareError(T),
}

/// The received message has an invalid header prefix.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidHeaderPrefix {
	pub actual: [u8; 4],
	pub expected: [u8; 4],
}

/// The received message has an invalid checksum value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidChecksum {
	pub message: u16,
	pub computed: u16,
}

/// The received message has an invalid or unexpected packet ID.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidPacketId {
	pub actual: u8,
	pub expected: Option<u8>,
}

/// The received message has an invalid or unexpected instruction value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidInstruction {
	pub actual: u8,
	pub expected: u8,
}

/// The expected number of parameters.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExpectedCount {
	Exact(usize),
	Max(usize),
}

/// The received message has an invalid or unexpected parameter count.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidParameterCount {
	pub actual: usize,
	pub expected: ExpectedCount,
}

impl<T> MotorError<T> {
	pub fn check(raw: u8, data: T) -> Result<T, Self> {
		if raw & !0x80 != 0 {
			Err(Self::MotorError {
				raw: 0, // Replace 0 with the appropriate value
			})
		} else if raw & 0x80 != 0 {
			Err(Self::HardwareError(data))
		} else {
			Ok(data)
		}
	}
}

impl InvalidHeaderPrefix {
	pub fn check(actual: &[u8], expected: [u8; 4]) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self {
				actual: [actual[0], actual[1], actual[2], actual[3]],
				expected,
			})
		}
	}
}

impl InvalidPacketId {
	pub fn check(actual: u8, expected: u8) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self {
				actual,
				expected: Some(expected),
			})
		}
	}

	pub fn check_ignore_broadcast(actual: u8, expected: u8) -> Result<(), Self> {
		if expected == BROADCAST {
			Ok(())
		} else {
			Self::check(actual, expected)
		}
	}
}

impl InvalidInstruction {
	pub fn check(actual: u8, expected: u8) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self { actual, expected })
		}
	}
}

impl InvalidParameterCount {
	pub fn check(actual: usize, expected: usize) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self {
				actual,
				expected: ExpectedCount::Exact(expected),
			})
		}
	}

	pub fn check_max(actual: usize, max: usize) -> Result<(), Self> {
		if actual <= max {
			Ok(())
		} else {
			Err(Self {
				actual,
				expected: ExpectedCount::Max(max),
			})
		}
	}
}

impl<T> std::error::Error for TransferError<T> where T: std::fmt::Debug {}
impl std::error::Error for WriteError {}
impl<T> std::error::Error for ReadError<T> where T: std::fmt::Debug {}
impl std::error::Error for InvalidMessage {}
impl<T> std::error::Error for MotorError<T> where T: std::fmt::Debug {}
impl std::error::Error for InvalidHeaderPrefix {}
impl std::error::Error for InvalidChecksum {}
impl std::error::Error for InvalidPacketId {}
impl std::error::Error for InvalidInstruction {}
impl std::error::Error for InvalidParameterCount {}

impl<ReadBuffer, WriteBuffer> From<TransferError<Response<'_, ReadBuffer, WriteBuffer>>> for TransferError<u8>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn from(other: TransferError<Response<ReadBuffer, WriteBuffer>>) -> Self {
		match other {
			TransferError::WriteError(e) => Self::WriteError(e),
			TransferError::ReadError(e) => Self::ReadError(match e {
				ReadError::Io(e) => ReadError::Io(e),
				ReadError::InvalidMessage(e) => ReadError::InvalidMessage(e),
				ReadError::MotorError(e) => ReadError::MotorError(match e {
					MotorError::MotorError { raw: e } => MotorError::MotorError { raw: e },
					MotorError::HardwareError(e) => MotorError::HardwareError(endian::read_u8_le(e.data())),
				}),
			}),
		}
	}
}

impl<ReadBuffer, WriteBuffer> From<TransferError<Response<'_, ReadBuffer, WriteBuffer>>> for TransferError<u16>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn from(other: TransferError<Response<ReadBuffer, WriteBuffer>>) -> Self {
		match other {
			TransferError::WriteError(e) => Self::WriteError(e),
			TransferError::ReadError(e) => Self::ReadError(match e {
				ReadError::Io(e) => ReadError::Io(e),
				ReadError::InvalidMessage(e) => ReadError::InvalidMessage(e),
				ReadError::MotorError(e) => ReadError::MotorError(match e {
					MotorError::MotorError { raw: e } => MotorError::MotorError { raw: e },
					MotorError::HardwareError(e) => MotorError::HardwareError(endian::read_u16_le(e.data())),
				}),
			}),
		}
	}
}

impl<ReadBuffer, WriteBuffer> From<TransferError<Response<'_, ReadBuffer, WriteBuffer>>> for TransferError<u32>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn from(other: TransferError<Response<ReadBuffer, WriteBuffer>>) -> Self {
		match other {
			TransferError::WriteError(e) => Self::WriteError(e),
			TransferError::ReadError(e) => Self::ReadError(match e {
				ReadError::Io(e) => ReadError::Io(e),
				ReadError::InvalidMessage(e) => ReadError::InvalidMessage(e),
				ReadError::MotorError(e) => ReadError::MotorError(match e {
					MotorError::MotorError { raw: e } => MotorError::MotorError { raw: e },
					MotorError::HardwareError(e) => MotorError::HardwareError(endian::read_u32_le(e.data())),
				}),
			}),
		}
	}
}

impl<T> From<WriteError> for TransferError<T> {
	fn from(other: WriteError) -> Self {
		Self::WriteError(other)
	}
}

impl<T> From<ReadError<T>> for TransferError<T> {
	fn from(other: ReadError<T>) -> Self {
		Self::ReadError(other)
	}
}

impl<T> From<std::io::Error> for ReadError<T> {
	fn from(other: std::io::Error) -> Self {
		Self::Io(other)
	}
}

impl<T> From<std::io::ErrorKind> for ReadError<T> {
	fn from(other: std::io::ErrorKind) -> Self {
		Self::Io(other.into())
	}
}

impl<T> From<InvalidMessage> for ReadError<T> {
	fn from(other: InvalidMessage) -> Self {
		Self::InvalidMessage(other)
	}
}

impl<T> From<MotorError<T>> for ReadError<T> {
	fn from(other: MotorError<T>) -> Self {
		Self::MotorError(other)
	}
}

impl<T> From<InvalidHeaderPrefix> for ReadError<T> {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<T> From<InvalidChecksum> for ReadError<T> {
	fn from(other: InvalidChecksum) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<T> From<InvalidPacketId> for ReadError<T> {
	fn from(other: InvalidPacketId) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<T> From<InvalidInstruction> for ReadError<T> {
	fn from(other: InvalidInstruction) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<T> From<InvalidParameterCount> for ReadError<T> {
	fn from(other: InvalidParameterCount) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl From<InvalidHeaderPrefix> for InvalidMessage {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::InvalidHeaderPrefix(other)
	}
}

impl From<InvalidChecksum> for InvalidMessage {
	fn from(other: InvalidChecksum) -> Self {
		Self::InvalidChecksum(other)
	}
}

impl From<InvalidPacketId> for InvalidMessage {
	fn from(other: InvalidPacketId) -> Self {
		Self::InvalidPacketId(other)
	}
}

impl From<InvalidInstruction> for InvalidMessage {
	fn from(other: InvalidInstruction) -> Self {
		Self::InvalidInstruction(other)
	}
}

impl From<InvalidParameterCount> for InvalidMessage {
	fn from(other: InvalidParameterCount) -> Self {
		Self::InvalidParameterCount(other)
	}
}

impl<T> std::fmt::Display for TransferError<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::WriteError(e) => write!(f, "{}", e),
			Self::ReadError(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for WriteError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::DiscardBuffer(e) => write!(f, "failed to discard input buffer: {}", e),
			Self::Write(e) => write!(f, "failed to write to serial port: {}", e),
		}
	}
}

impl<T> std::fmt::Display for ReadError<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Io(e) => write!(f, "failed to read from serial port: {}", e),
			Self::InvalidMessage(e) => write!(f, "{}", e),
			Self::MotorError(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for InvalidMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidHeaderPrefix(e) => write!(f, "{}", e),
			Self::InvalidChecksum(e) => write!(f, "{}", e),
			Self::InvalidPacketId(e) => write!(f, "{}", e),
			Self::InvalidInstruction(e) => write!(f, "{}", e),
			Self::InvalidParameterCount(e) => write!(f, "{}", e),
		}
	}
}

impl<T> std::fmt::Display for MotorError<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::MotorError { raw } => write!(f, "motor reported error status: {:#02X}", raw),
			Self::HardwareError(_) => write!(f, "hardware error reported"),
		}
	}
}

impl std::fmt::Display for InvalidHeaderPrefix {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"invalid header prefix, expected {:02X?}, got {:02X?}",
			self.expected, self.actual
		)
	}
}

impl std::fmt::Display for InvalidChecksum {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"invalid checksum, message claims {:#02X}, computed {:#02X}",
			self.message, self.computed
		)
	}
}

impl std::fmt::Display for InvalidPacketId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if let Some(expected) = self.expected {
			write!(f, "invalid packet ID, expected {:#02X}, got {:#02X}", expected, self.actual)
		} else {
			write!(f, "invalid packet ID: {:#02X}", self.actual)
		}
	}
}

impl std::fmt::Display for InvalidInstruction {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"invalid instruction ID, expected {:#02X}, got {:#02X}",
			self.expected, self.actual
		)
	}
}

impl std::fmt::Display for ExpectedCount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Exact(x) => write!(f, "exactly {}", x),
			Self::Max(x) => write!(f, "at most {}", x),
		}
	}
}

impl std::fmt::Display for InvalidParameterCount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid parameter count, expected {}, got {}", self.expected, self.actual)
	}
}
