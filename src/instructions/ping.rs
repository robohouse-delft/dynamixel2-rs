use super::{instruction_id, packet_id};
use crate::bus::StatusPacket;
use crate::{Bus, ReadError, TransferError, WriteError};

#[derive(Debug)]
pub struct PingResponse {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The model of the motor.
	///
	/// Refer to the online manual to find the codes for each model.
	pub model: u16,

	/// The firmware version of the motor.
	pub firmware: u8,

	pub alert: bool,
}

impl<'a, ReadBuffer, WriteBuffer> TryFrom<StatusPacket<'a, ReadBuffer, WriteBuffer>> for PingResponse
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Result<Self, Self::Error> {
		let parameters = status_packet.parameters();
		crate::InvalidParameterCount::check(parameters.len(), 3)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			model: crate::endian::read_u16_le(&parameters[0..]),
			firmware: crate::endian::read_u8_le(&parameters[2..]),
			alert: status_packet.alert(),
		})
	}
}

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Ping a specific motor by ID.
	///
	/// This will not work correctly if the motor ID is [`packet_id::BROADCAST`].
	/// Use [`Self::scan`] or [`Self::scan_cb`] instead.
	pub fn ping(&mut self, motor_id: u8) -> Result<PingResponse, TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::PING, 0, |_| ())?;
		Ok(response.try_into()?)
	}

	/// Scan a bus for motors with a broadcast ping, returning the responses in a [`Vec`].
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are collected.
	pub fn scan(&mut self) -> Result<Vec<Result<PingResponse, ReadError>>, WriteError> {
		let mut result = Vec::with_capacity(253);
		self.scan_cb(|x| result.push(x))?;
		Ok(result)
	}

	/// Scan a bus for motors with a broadcast ping, calling an [`FnMut`] for each response.
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are passed to the handler.
	pub fn scan_cb<F>(&mut self, mut on_response: F) -> Result<(), WriteError>
	where
		F: FnMut(Result<PingResponse, ReadError>),
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::PING, 0, |_| ())?;

		// TODO: See if we can terminate quicker.
		// Peek at the official SDK to see what they do.

		for _ in 0..253 {
			let response = self.read_status_response();
			if let Err(ReadError::Io(e)) = &response {
				if e.kind() == std::io::ErrorKind::TimedOut {
					trace!("Response timed out.");
					continue;
				}
			}
			let response = response.and_then(|response| Ok(response.try_into()?));
			on_response(response);
		}

		Ok(())
	}
}
