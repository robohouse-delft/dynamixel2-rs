use crate::endian::write_u16_le;
use crate::{Bus, ReadError, WriteError};
use super::{instruction_id, packet_id, ReadResponse};

/// The responses to a [`Bus::sync_read`] command.
///
/// This struct allows you to read the responses sent by each motor.
/// However, those responses are not actually read until you call [`Self::read_next`].
/// You should always read all responses to avoid the risk of bus collisions.
///
/// Additionally, you must not wait any significant time before reading each response,
/// or you risk the OS throwing away unread data from the serial port.
#[must_use = "This struct represents messages that have not yet been read. Failing to read all messages may cause bus collisions."]
pub struct SyncReadResponse<'a, Stream, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	bus: &'a mut crate::Bus<Stream, ReadBuffer, WriteBuffer>,
	motor_ids: &'a [u8],
	parameter_count: u16,
	received: usize,
}

impl<'a, Stream, ReadBuffer, WriteBuffer> SyncReadResponse<'a, Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read the reply from the next motor until none are left.
	///
	/// You should always read all responses to avoid the risk of bus collisions.
	/// This can be done by calling this function until it returns [`None`].
	pub fn read_next(&mut self) -> Option<Result<ReadResponse<Stream, ReadBuffer, WriteBuffer>, ReadError>> {
		if self.received == self.motor_ids.len() {
			None
		} else {
			Some(self.read_next_())
		}
	}

	pub fn read_next_(&mut self) -> Result<ReadResponse<Stream, ReadBuffer, WriteBuffer>, ReadError> {
		let motor_id = self.motor_ids[self.received];
		let response = self.bus.read_status_response()?;
		self.received += 1;
		crate::error::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), self.parameter_count.into())?;
		Ok(ReadResponse { response })
	}
}

impl<Stream, ReadBuffer, WriteBuffer> Bus<Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read an arbitrary number of bytes from multiple motors in one command.
	///
	/// The data returned by each motor is sampled when the motor receives the command, not when the reply is sent.
	/// This is useful to read the status of multiple motors at a single point in time.
	///
	/// The returned struct allows you to read the responses sent by each motor.
	/// However, those responses are not actually read until you call [`SyncReadResponse::read_next`] to exhaustion.
	/// You should always read all responses to avoid the risk of bus collisions.
	///
	/// Additionally, you must not wait any significant time before reading each response,
	/// or you risk the OS throwing away unread data from the serial port.
	pub fn sync_read<'a>(&'a mut self, motor_ids: &'a [u8], address: u16, count: u16) -> Result<SyncReadResponse<'a, Stream, ReadBuffer, WriteBuffer>, WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
		})?;
		Ok(SyncReadResponse {
			bus: self,
			motor_ids,
			parameter_count: count,
			received: 0,
		})
	}
}
