#[macro_use]
mod log;

pub mod crc;
pub mod instructions;

mod bitstuff;
mod endian;
mod error;
mod transfer;

pub use error::InvalidChecksum;
pub use error::InvalidHeaderPrefix;
pub use error::InvalidInstruction;
pub use error::InvalidMessage;
pub use error::InvalidPacketId;
pub use error::InvalidParameterCount;
pub use error::ReadError;

pub use transfer::read_response;
pub use transfer::write_request;
