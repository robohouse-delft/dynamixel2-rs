use core::time::Duration;

use super::{instruction_id, packet_id};
use crate::serial_port::SerialPort;
use crate::{Bus, ReadError, Response, TransferError, StatusPacket};

use crate::packet::Packet;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// A response from a motor to a ping instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Ping {
	/// The model of the motor.
	///
	/// Refer to the online manual to find the codes for each model.
	pub model: u16,

	/// The firmware version of the motor.
	pub firmware: u8,
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<Ping> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		let parameters = status_packet.parameters();
		crate::InvalidParameterCount::check(parameters.len(), 3)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: Ping {
				model: crate::endian::read_u16_le(&parameters[0..]),
				firmware: crate::endian::read_u8_le(&parameters[2..]),
			},
		})
	}
}

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Ping a specific motor by ID.
	///
	/// This will not work correctly if the motor ID is [`packet_id::BROADCAST`].
	/// Use [`Self::scan`] or [`Self::scan_cb`] instead.
	pub fn ping(&mut self, motor_id: u8) -> Result<Response<Ping>, TransferError<T::Error>> {
		let response = self.transfer_single(motor_id, instruction_id::PING, 0, 3, |_| ())?;
		Ok(response.try_into()?)
	}

	/// Scan a bus for motors with a broadcast ping, returning the responses in a [`Vec`].
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are collected.
	#[cfg(any(feature = "alloc", feature = "std"))]
	pub fn scan(&mut self) -> Result<Vec<Result<Response<Ping>, ReadError<T::Error>>>, crate::WriteError<T::Error>> {
		let mut result = Vec::with_capacity(253);
		match self.scan_cb(|x| result.push(Ok(x))) {
			Ok(()) => (),
			Err(TransferError::WriteError(e)) => return Err(e),
			Err(TransferError::ReadError(e)) => {
				result.push(Err(e));
			},
		}
		Ok(result)
	}

	/// Scan a bus for motors with a broadcast ping, calling an [`FnMut`] for each response.
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are passed to the handler.
	pub fn scan_cb<F>(&mut self, mut on_response: F) -> Result<(), TransferError<T::Error>>
	where
		F: FnMut(Response<Ping>),
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::PING, 0, |_| ())?;
		let response_time = crate::bus::message_transfer_time(14, self.baud_rate());
		let timeout = response_time * 253 + Duration::from_millis(34);

		loop {
			let response = self.read_status_response_timeout(timeout);
			match response {
				Ok(response) => {
					let response = response.try_into()?;
					on_response(response);
				},
				Err(ReadError::Io(e)) if T::is_timeout_error(&e) => {
					trace!("Ping response timed out.");
					return Ok(());
				},
				Err(e) => {
					return Err(e.into());
				}
			}
		}
	}
}
