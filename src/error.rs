use crate::instructions::packet_id::BROADCAST;
use crate::instructions::InstructionId;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// An error that can occur while initializing a bus.
#[derive(Debug)]
pub enum InitializeError<E> {
	/// Failed to get the configuration of the serial port.
	GetConfiguration(E),

	/// Failed to get the baud rate from the configuration of the serial port.
	GetBaudRate(E),
}

/// An error that can occur during a read/write transfer.
#[derive(Debug)]
pub enum TransferError<E> {
	/// The write of failed.
	WriteError(WriteError<E>),

	/// The read failed.
	ReadError(ReadError<E>),
}

/// An error that can occur during a write transfer.
#[derive(Debug)]
pub enum WriteError<E> {
	/// The write buffer is too small to contain the whole stuffed message.
	BufferTooSmall(BufferTooSmallError),

	/// Failed to discard the input buffer before writing the instruction.
	DiscardBuffer(E),

	/// Failed to write the instruction.
	Write(E),
}

/// The buffer is too small to hold the entire message.
///
/// Consider increasing the size of the buffer.
/// Keep in mind that the write buffer needs to be large enough to account for byte stuffing.
#[derive(Debug)]
pub struct BufferTooSmallError {
	/// The required size of the buffer.
	pub required_size: usize,

	/// The total size of the buffer.
	pub total_size: usize,
}

/// An error that can occur during a read transfer.
#[derive(Debug)]
pub enum ReadError<E> {
	/// The read buffer is too small to contain the whole stuffed message.
	BufferFull(BufferTooSmallError),

	/// Failed to read from the serial port.
	Io(E),

	/// A timeout occurred while waiting for a response.
	Timeout,

	/// The received message is invalid.
	InvalidMessage(InvalidMessage),

	/// The motor reported an error instead of a valid response.
	///
	/// This error is not returned when a motor signals a hardware error,
	/// since the instruction has still been exectuted.
	///
	/// Instead, the `alert` bit in the response will be set.
	MotorError(MotorError),
}

/// The received message is not valid.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InvalidMessage {
	/// The header does not start with the proper prefix.
	InvalidHeaderPrefix(InvalidHeaderPrefix),

	/// The message checksum is invalid.
	InvalidChecksum(InvalidChecksum),

	/// The message has an invalid packet ID.
	InvalidPacketId(InvalidPacketId),

	/// The message has an invalid instruction.
	InvalidInstruction(InvalidInstruction),

	/// The message has an invalid parameter count.
	InvalidParameterCount(InvalidParameterCount),
}

/// An error reported by the motor.
#[derive(Clone, Eq, PartialEq)]
pub struct MotorError {
	/// The raw error as returned by the motor.
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

impl Debug for MotorError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("MotorError")
			.field("error_number", &self.error_number())
			.field("alert", &self.alert())
			.finish()
	}
}

/// The received message has an invalid header prefix.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidHeaderPrefix {
	/// The actual prefix.
	pub actual: [u8; 4],

	/// The expected prefix.
	pub expected: [u8; 4],
}

/// The received message has an invalid checksum value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidChecksum {
	/// The checksum from the messsage.
	pub message: u16,

	/// The actual checksum.
	pub computed: u16,
}

/// The received message has an invalid or unexpected packet ID.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidPacketId {
	/// The actual packet ID.
	pub actual: u8,

	/// The expected packet ID (if a specific ID was expected).
	pub expected: Option<u8>,
}

/// The received message has an invalid or unexpected instruction value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidInstruction {
	/// The actual instruction ID.
	pub actual: InstructionId,

	/// The expected instruction id.
	pub expected: InstructionId,
}

/// The expected number of parameters.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExpectedCount {
	/// The exact number of expected parameters.
	Exact(usize),

	/// An upper limit on the expected number of parameters.
	Max(usize),
}

/// The received message has an invalid or unexpected parameter count.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidParameterCount {
	/// The actual parameter count.
	pub actual: usize,

	/// The expected parameter count.
	pub expected: ExpectedCount,
}

impl BufferTooSmallError {
	/// Check if a buffer is large enough for the required total size.
	pub fn check(required_size: usize, total_size: usize) -> Result<(), Self> {
		if required_size <= total_size {
			Ok(())
		} else {
			Err(Self { required_size, total_size })
		}
	}
}

impl MotorError {
	/// Check for a motor error in the response.
	///
	/// This ignores the `alert` bit,
	/// since it indicates a hardware error and not a failed instruction.
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
	/// Check if the header prefix matches the expected value.
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
	/// Check if the packet ID matches the expected value.
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

	/// Check if the packet ID matches the expected value, but don't report an error if the ID is the broadcast ID.
	pub fn check_ignore_broadcast(actual: u8, expected: u8) -> Result<(), Self> {
		if expected == BROADCAST {
			Ok(())
		} else {
			Self::check(actual, expected)
		}
	}
}

impl InvalidInstruction {
	/// Check if the instruction ID is the expected value.
	pub fn check(actual: InstructionId, expected: InstructionId) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self { actual, expected })
		}
	}
}

impl InvalidParameterCount {
	/// Check if the parameter count matches the expected count.
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

	/// Check if the parameter count is at or below the max count.
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

#[cfg(feature = "std")]
impl<E: Debug + Display> std::error::Error for InitializeError<E> {}
#[cfg(feature = "std")]
impl<E: Debug + Display> std::error::Error for TransferError<E> {}
#[cfg(feature = "std")]
impl<E: Debug + Display> std::error::Error for WriteError<E> {}
#[cfg(feature = "std")]
impl<E: Debug + Display> std::error::Error for ReadError<E> {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidMessage {}
#[cfg(feature = "std")]
impl std::error::Error for MotorError {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidHeaderPrefix {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidChecksum {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidPacketId {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidInstruction {}
#[cfg(feature = "std")]
impl std::error::Error for InvalidParameterCount {}

impl<E> From<WriteError<E>> for TransferError<E>
{
	fn from(other: WriteError<E>) -> Self {
		Self::WriteError(other)
	}
}

impl<E> From<ReadError<E>> for TransferError<E>
{
	fn from(other: ReadError<E>) -> Self {
		Self::ReadError(other)
	}
}

impl<E> From<InvalidMessage> for TransferError<E> {
	fn from(other: InvalidMessage) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<InvalidHeaderPrefix> for TransferError<E> {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<InvalidChecksum> for TransferError<E> {
	fn from(other: InvalidChecksum) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<InvalidPacketId> for TransferError<E> {
	fn from(other: InvalidPacketId) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<InvalidInstruction> for TransferError<E> {
	fn from(other: InvalidInstruction) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<InvalidParameterCount> for TransferError<E> {
	fn from(other: InvalidParameterCount) -> Self {
		Self::ReadError(other.into())
	}
}

impl<E> From<BufferTooSmallError> for WriteError<E> {
	fn from(other: BufferTooSmallError) -> Self {
		Self::BufferTooSmall(other)
	}
}

impl<E> From<BufferTooSmallError> for ReadError<E> {
	fn from(other: BufferTooSmallError) -> Self {
		Self::BufferFull(other)
	}
}

impl<E> From<InvalidMessage> for ReadError<E> {
	fn from(other: InvalidMessage) -> Self {
		Self::InvalidMessage(other)
	}
}

impl<E> From<MotorError> for ReadError<E> {
	fn from(other: MotorError) -> Self {
		Self::MotorError(other)
	}
}

impl<E> From<InvalidHeaderPrefix> for ReadError<E> {
	fn from(other: InvalidHeaderPrefix) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<E> From<InvalidChecksum> for ReadError<E> {
	fn from(other: InvalidChecksum) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<E> From<InvalidPacketId> for ReadError<E> {
	fn from(other: InvalidPacketId) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<E> From<InvalidInstruction> for ReadError<E> {
	fn from(other: InvalidInstruction) -> Self {
		Self::InvalidMessage(other.into())
	}
}

impl<E> From<InvalidParameterCount> for ReadError<E> {
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

impl<E> Display for InitializeError<E>
where
	E: Display,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::GetConfiguration(e) => write!(f, "failed to get configuration of serial port: {e}"),
			Self::GetBaudRate(e) => write!(f, "failed to get baud rate of serial port: {e}"),
		}
	}
}

impl<E> Display for TransferError<E>
where
	E: Display,
{
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Self::WriteError(e) => write!(f, "{}", e),
			Self::ReadError(e) => write!(f, "{}", e),
		}
	}
}

impl<E> Display for WriteError<E>
where
	E: Display,
{
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Self::BufferTooSmall(e) => write!(
				f,
				"write buffer is too small: need {} bytes, but the size is {}",
				e.required_size, e.total_size
			),
			Self::DiscardBuffer(e) => write!(f, "failed to discard input buffer: {}", e),
			Self::Write(e) => write!(f, "failed to write to serial port: {}", e),
		}
	}
}

impl Display for BufferTooSmallError {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(
			f,
			"buffer is too small: need {} bytes, but the size is {}",
			self.required_size, self.total_size
		)
	}
}

impl<E> Display for ReadError<E>
where
	E: Display,
{
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Self::BufferFull(e) => write!(
				f,
				"read buffer is too small: need {} bytes, but the size is {}",
				e.required_size, e.total_size
			),
			Self::Io(e) => write!(f, "failed to read from serial port: {}", e),
			Self::Timeout => write!(f, "timeout while waiting for response"),
			Self::InvalidMessage(e) => write!(f, "{}", e),
			Self::MotorError(e) => write!(f, "{}", e),
		}
	}
}

impl Display for InvalidMessage {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Self::InvalidHeaderPrefix(e) => write!(f, "{}", e),
			Self::InvalidChecksum(e) => write!(f, "{}", e),
			Self::InvalidPacketId(e) => write!(f, "{}", e),
			Self::InvalidInstruction(e) => write!(f, "{}", e),
			Self::InvalidParameterCount(e) => write!(f, "{}", e),
		}
	}
}

impl Display for MotorError {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "motor reported error status: {:#02X}", self.raw,)
	}
}

impl Display for InvalidHeaderPrefix {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(
			f,
			"invalid header prefix, expected {:02X?}, got {:02X?}",
			self.expected, self.actual
		)
	}
}

impl Display for InvalidChecksum {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(
			f,
			"invalid checksum, message claims {:#02X}, computed {:#02X}",
			self.message, self.computed
		)
	}
}

impl Display for InvalidPacketId {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		if let Some(expected) = self.expected {
			write!(f, "invalid packet ID, expected {:#02X}, got {:#02X}", expected, self.actual)
		} else {
			write!(f, "invalid packet ID: {:#02X}", self.actual)
		}
	}
}

impl Display for InvalidInstruction {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(
			f,
			"invalid instruction ID, expected {:?}, got {:?}",
			self.expected, self.actual
		)
	}
}

impl Display for ExpectedCount {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Self::Exact(x) => write!(f, "exactly {}", x),
			Self::Max(x) => write!(f, "at most {}", x),
		}
	}
}

impl Display for InvalidParameterCount {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "invalid parameter count, expected {}, got {}", self.expected, self.actual)
	}
}
