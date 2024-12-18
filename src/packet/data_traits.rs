use crate::{InvalidMessage, InvalidParameterCount, Packet, Response, StatusPacket};
use crate::endian::{read_u16_le, read_u32_le, read_u8_le};

pub trait Write {
	const W_COUNT: u16;

	fn write_bytes(&self, buffer: impl AsMut<[u8]>);
}

impl Write for u8 {
	const W_COUNT: u16 = size_of::<u8>() as u16;
	fn write_bytes(&self, mut buffer: impl AsMut<[u8]>) {
		buffer.as_mut()[0] = *self;
	}
}

impl Write for u16 {
	const W_COUNT: u16 = size_of::<u16>() as u16;
	fn write_bytes(&self, mut buffer: impl AsMut<[u8]>) {
		buffer.as_mut().copy_from_slice(&self.to_le_bytes());
	}
}

impl Write for u32 {
	const W_COUNT: u16 = size_of::<u32>() as u16;
	fn write_bytes(&self, mut buffer: impl AsMut<[u8]>) {
		buffer.as_mut().copy_from_slice(&self.to_le_bytes());
	}
}

impl<const N: usize> Write for [u8; N] {
	const W_COUNT: u16 = N as u16;

	fn write_bytes(&self, mut buffer: impl AsMut<[u8]>) {
		buffer.as_mut().copy_from_slice(self)
	}
}

pub trait Read {
	const R_COUNT: u16;

	fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidMessage> where Self: Sized;

	fn response_from_status(status_packet: StatusPacket<'_>) -> Result<Response<Self>, InvalidMessage> where Self: Sized {
		let data = Self::try_from_bytes(status_packet.parameters())?;
		Ok(Response {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data,
		})
	}
}

impl Read for u8 {
	const R_COUNT: u16 = size_of::<u8>() as u16;

	fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidMessage> {
		InvalidParameterCount::check(bytes.len(), Self::R_COUNT as usize)?;
		Ok(read_u8_le(bytes))
	}
}

impl Read for u16 {
	const R_COUNT: u16 = size_of::<u16>() as u16;

	fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidMessage> {
		InvalidParameterCount::check(bytes.len(), Self::R_COUNT as usize)?;
		Ok(read_u16_le(bytes))
	}
}

impl Read for u32 {
	const R_COUNT: u16 = size_of::<u32>() as u16;

	fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidMessage> {
		InvalidParameterCount::check(bytes.len(), Self::R_COUNT as usize)?;
		Ok(read_u32_le(bytes))

	}
}

impl<const N: usize> Read for [u8; N] {
	const R_COUNT: u16 = N as u16;
	fn try_from_bytes(bytes: &[u8]) -> Result<Self, InvalidMessage> {
		InvalidParameterCount::check(bytes.len(), Self::R_COUNT as usize)?;
		let r = bytes.try_into().map_err(|_| InvalidMessage::ParseError)?;
		Ok(r)
	}
}