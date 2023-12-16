use crate::instructions::packet_id::BROADCAST;

/// An error that can occur during a read/write transfer.
#[derive(Debug)]
pub enum TransferError {
	WriteError(WriteError),
	ReadError(ReadError),
}

/// An error that can occur during a write transfer.
#[derive(Debug)]
pub enum WriteError {
	DiscardBuffer(std::io::Error),
	Write(std::io::Error),
}

/// An error that can occur during a read transfer.
#[derive(Debug)]
pub enum ReadError {
	Io(std::io::Error),
	InvalidMessage(InvalidMessage),
	MotorError(MotorError),
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
pub struct MotorError {
	pub raw: u8,
}

impl MotorError {
	/// The error number reported by the motor.
	///
	/// This is the lower 7 bits of the raw error field.
	pub fn error_number(&self) -> u8 {
		self.raw & !0x80
	}

	/// The alert bit from the error field of the response.
	///
	/// This is the 8th bit of the raw error field.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub fn alert(&self) -> bool {
		self.raw & 0x80 != 0
	}
}

impl std::fmt::Debug for MotorError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MotorError")
			.field("error_number", &self.error_number())
			.field("alert", &self.alert())
			.finish()
	}

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

impl MotorError {
	pub fn check(raw: u8) -> Result<(), Self> {
		// Ignore the alert bit for this check.
		// If the alert bit is set, the motor encountered an error, but the instruction was still executed.
		if raw & !0x80 == 0 {
			Ok(())
		} else {
			Err(Self { raw })
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

impl std::error::Error for TransferError {}
impl std::error::Error for WriteError {}
impl std::error::Error for ReadError {}
impl std::error::Error for InvalidMessage {}
impl std::error::Error for MotorError {}
impl std::error::Error for InvalidHeaderPrefix {}
impl std::error::Error for InvalidChecksum {}
impl std::error::Error for InvalidPacketId {}
impl std::error::Error for InvalidInstruction {}
impl std::error::Error for InvalidParameterCount {}

impl From<WriteError> for TransferError {
	fn from(other: WriteError) -> Self {
		Self::WriteError(other)
	}
}

impl From<ReadError> for TransferError {
	fn from(other: ReadError) -> Self {
		Self::ReadError(other)
	}
}

impl From<InvalidMessage> for TransferError {
	fn from(other: InvalidMessage) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<InvalidHeaderPrefix> for TransferError {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<InvalidChecksum> for TransferError {
	fn from(other: InvalidChecksum) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<InvalidPacketId> for TransferError {
	fn from(other: InvalidPacketId) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<InvalidInstruction> for TransferError {
	fn from(other: InvalidInstruction) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<InvalidParameterCount> for TransferError {
	fn from(other: InvalidParameterCount) -> Self {
		Self::ReadError(other.into())
	}
}

impl From<std::io::Error> for ReadError {
	fn from(other: std::io::Error) -> Self {
		Self::Io(other)
	}
}

impl From<std::io::ErrorKind> for ReadError {
	fn from(other: std::io::ErrorKind) -> Self {
		Self::Io(other.into())
	}
}

impl From<InvalidMessage> for ReadError {
	fn from(other: InvalidMessage) -> Self {
		Self::InvalidMessage(other)
	}
}

impl From<MotorError> for ReadError {
	fn from(other: MotorError) -> Self {
		Self::MotorError(other)
	}
}

impl From<InvalidHeaderPrefix> for ReadError {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl From<InvalidChecksum> for ReadError {
	fn from(other: InvalidChecksum) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl From<InvalidPacketId> for ReadError {
	fn from(other: InvalidPacketId) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl From<InvalidInstruction> for ReadError {
	fn from(other: InvalidInstruction) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl From<InvalidParameterCount> for ReadError {
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

impl std::fmt::Display for TransferError {
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

impl std::fmt::Display for ReadError {
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

impl std::fmt::Display for MotorError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "motor reported error status: {:#02X}", self.raw,)
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
