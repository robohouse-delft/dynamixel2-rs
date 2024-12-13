use crate::endian::{read_u16_le, read_u32_le, read_u8_le};
use crate::Packet;
use crate::packet::STATUS_HEADER_SIZE;
use crate::packet::data_traits::Read;

/// A status response that is currently in the read buffer of a bus.
///
/// When dropped, the response data is removed from the read buffer.
#[derive(Debug)]
pub struct StatusPacket<'a> {
	/// Message data (with byte-stuffing already undone).
	pub(crate) data: &'a [u8],
}

impl StatusPacket<'_> {
	/// The error field of the response.
	pub fn error(&self) -> u8 {
		self.as_bytes()[8]
	}

	/// The error number of the status packet.
	///
	/// This is the lower 7 bits of the error field.
	pub fn error_number(&self) -> u8 {
		self.error() & !0x80
	}

	/// The alert bit from the error field of the response.
	///
	/// This is the 8th bit of the error field.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub fn alert(&self) -> bool {
		self.error() & 0x80 != 0
	}

	pub fn try_into_response<T>(self) -> Result<Response<T>, crate::InvalidMessage> where T: Read {
		Ok(
			Response {
				motor_id: self.packet_id(),
				alert: self.alert(),
				data:  T::try_from_bytes(self.data)?,
			}
		)
	}

	pub fn as_response(&self) -> Response<&[u8]> {
		Response {
			motor_id: self.packet_id(),
			alert: self.alert(),
			data: self.parameters()
		}
	}
}

/// A response from a motor.
///
/// Note that the `Eq` and `PartialEq` compare all fields of the struct,
/// including the `motor_id` and `alert`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Response<T> {
	/// The motor that sent the response.
	pub motor_id: u8,

	/// The alert bit from the response message.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub alert: bool,

	/// The data from the motor.
	pub data: T,
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<()> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameters().len(), 0)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: (),
		})
	}
}

impl<'a> From<StatusPacket<'a>> for Response<&'a [u8]> {
	fn from(status_packet: StatusPacket<'a>) -> Self {
		Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: &status_packet.data[STATUS_HEADER_SIZE..],
		}
	}
}

impl<'a, 'b> From<&'b StatusPacket<'a>> for Response<&'b [u8]> {
	fn from(status_packet: &'b StatusPacket<'a>) -> Self {
		Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: status_packet.parameters(),
		}
	}
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl<'a> From<StatusPacket<'a>> for Response<Vec<u8>> {
	fn from(status_packet: StatusPacket<'a>) -> Self {
		Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: status_packet.parameters().to_owned(),
		}
	}
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<u8> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameters().len(), 1)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u8_le(status_packet.parameters()),
		})
	}
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<u16> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameters().len(), 2)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u16_le(status_packet.parameters()),
		})
	}
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<u32> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameters().len(), 4)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u32_le(status_packet.parameters()),
		})
	}
}
//conflicint impls
// impl<'a, D> TryFrom<StatusPacket<'a>> for Response<D>
// where D: Read
// {
// 	type Error = crate::InvalidParameterCount;
//
// 	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
// 		crate::InvalidParameterCount::check(status_packet.parameters().len(), 4)?;
// 		let data = D::from_bytes(status_packet.parameters())?;
// 		Ok(Self {
// 			motor_id: status_packet.packet_id(),
// 			alert: status_packet.alert(),
// 			data
// 		})
// 	}
// }

impl<'a> From<Response<&'a [u8]>> for Response<Vec<u8>> {
	fn from(response: Response<&'a [u8]>) -> Self {
		Response {
			motor_id: response.motor_id,
			alert: response.alert,
			data: response.data.to_vec(),
		}
	}
}
