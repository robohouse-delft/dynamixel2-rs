#[derive(Debug)]
pub enum ReadError {
	Io(std::io::Error),
	InvalidMessage(InvalidMessage),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InvalidMessage {
	InvalidHeaderPrefix(InvalidHeaderPrefix),
	InvalidChecksum(InvalidChecksum),
	InvalidPacketId(InvalidPacketId),
	InvalidInstruction(InvalidInstruction),
	InvalidParameterCount(InvalidParameterCount),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidHeaderPrefix {
	pub actual: [u8; 4],
	pub expected: [u8; 4],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidChecksum {
	pub actual: u16,
	pub expected: u16,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidPacketId {
	pub actual: u8,
	pub expected: u8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidInstruction {
	pub actual: u8,
	pub expected: u8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidParameterCount {
	pub actual: usize,
	pub expected: usize,
}

impl InvalidHeaderPrefix {
	pub fn check(actual: &[u8], expected: [u8; 4]) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self {
				actual: [
					actual[0],
					actual[1],
					actual[2],
					actual[3],
				],
				expected,
			})
		}
	}
}

impl InvalidChecksum {
	pub fn check(actual: u16, expected: u16) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self { actual, expected })
		}
	}
}

impl InvalidPacketId {
	pub fn check(actual: u8, expected: u8) -> Result<(), Self> {
		if actual == expected {
			Ok(())
		} else {
			Err(Self { actual, expected })
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
			Err(Self { actual, expected })
		}
	}
}

impl From<std::io::Error> for ReadError {
	fn from(other: std::io::Error) -> Self {
		Self::Io(other)
	}
}

impl std::error::Error for ReadError {}
impl std::error::Error for InvalidMessage {}
impl std::error::Error for InvalidHeaderPrefix {}
impl std::error::Error for InvalidChecksum {}
impl std::error::Error for InvalidPacketId {}
impl std::error::Error for InvalidInstruction {}
impl std::error::Error for InvalidParameterCount {}

impl From<InvalidMessage> for ReadError {
	fn from(other: InvalidMessage) -> Self {
		Self::InvalidMessage(other)
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

impl std::fmt::Display for ReadError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Io(e) => write!(f, "{}", e),
			Self::InvalidMessage(e) => write!(f, "{}", e),
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

impl std::fmt::Display for InvalidHeaderPrefix {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid header prefix, expected {:02X?}, got {:02X?}", self.expected, self.actual)
	}
}

impl std::fmt::Display for InvalidChecksum {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid checksum, message claims {:#02X}, computed {:#02X}", self.expected, self.actual)
	}
}

impl std::fmt::Display for InvalidPacketId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid packet ID, expected {:#02X}, got {:#02X}", self.expected, self.actual)
	}
}

impl std::fmt::Display for InvalidInstruction {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid instruction ID, expected {:#02X}, got {:#02X}", self.expected, self.actual)
	}
}

impl std::fmt::Display for InvalidParameterCount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid parameter count, expected {:#02X}, got {:#02X}", self.expected, self.actual)
	}
}
