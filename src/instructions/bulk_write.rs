use super::{instruction_id, packet_id, BulkWriteData};
use crate::endian::{write_u16_le, write_u8_le};
use crate::systems::SerialPort;
use crate::{Bus, WriteError};

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,

	T: SerialPort,
{
	/// Synchronously write arbitrary data ranges to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	/// Unlike the sync write instruction, a bulk write allows you to write a different amount of data to a different address for each motor.
	///
	/// The data for multi-byte registers should serialized as little-endian.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one write.
	///
	/// This function also panics if the data length for a motor exceeds the capacity of a `u16`.
	///
	/// # Example
	/// ```no_run
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// use dynamixel2::Bus;
	/// use dynamixel2::instructions::BulkWriteData;
	/// use std::time::Duration;
	///
	/// let mut bus = Bus::open("/dev/ttyUSB0", 57600)?;
	/// bus.bulk_write(&[
	///   // Write a u32 value of 2000 to register 116 of motor 1.
	///   BulkWriteData {
	///     motor_id: 1,
	///     address: 116,
	///     data: 2000u32.to_le_bytes().as_slice(),
	///   },
	///   // Write a u16 value of 300 to register 102 of motor 2.
	///   BulkWriteData {
	///     motor_id: 2,
	///     address: 102,
	///     data: 300u16.to_le_bytes().as_slice(),
	///   },
	/// ])?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn bulk_write<'a, I, D>(&mut self, writes: &'a I) -> Result<(), WriteError<T::Error>>
	where
		&'a I: IntoIterator,
		<&'a I as IntoIterator>::IntoIter: Clone,
		<&'a I as IntoIterator>::Item: std::borrow::Borrow<BulkWriteData<D>>,
		D: AsRef<[u8]>,
	{
		use std::borrow::Borrow;

		let writes = writes.into_iter();
		let mut parameter_count = 0;
		for write in writes.clone() {
			let write = write.borrow();
			let data = write.data.as_ref();
			if data.len() > u16::MAX.into() {
				panic!(
					"bulk_write: data length ({}) for motor {} exceeds maximum size of {}",
					data.len(),
					write.motor_id,
					u16::MAX
				);
			}
			parameter_count += 5 + data.len();
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::BULK_WRITE, parameter_count, |buffer| {
			let mut offset = 0;
			for write in writes {
				let write = write.borrow();
				let data = write.data.as_ref();
				let buffer = &mut buffer[offset..];
				offset += 5 + data.len();
				write_u8_le(&mut buffer[0..], write.motor_id);
				write_u16_le(&mut buffer[1..], write.address);
				write_u16_le(&mut buffer[3..], data.len() as u16);
				buffer[5..][..data.len()].copy_from_slice(data);
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Ensure that `bulk_write` accepts a slice of `BulkWriteData`.
	///
	/// This is a compile test. It only tests that the test code compiles.
	#[allow(dead_code)]
	fn bulk_write_accepts_slice(bus: &mut Bus<Vec<u8>, Vec<u8>, serial2::SerialPort>) -> Result<(), Box<dyn std::error::Error>> {
		bus.bulk_write(&[
			BulkWriteData {
				motor_id: 1,
				address: 116,
				data: 2000u32.to_le_bytes().as_slice(),
			},
			BulkWriteData {
				motor_id: 2,
				address: 102,
				data: 300u16.to_le_bytes().as_slice(),
			},
		])?;
		Ok(())
	}

	/// Ensure that `bulk_write` accepts a reference to a Vec of `BulkWriteData`.
	///
	/// This is a compile test. It only tests that the test code compiles.
	#[allow(dead_code)]
	fn bulk_write_accepts_vec_ref(bus: &mut Bus<Vec<u8>, Vec<u8>, serial2::SerialPort>) -> Result<(), Box<dyn std::error::Error>> {
		bus.bulk_write(&vec![
			BulkWriteData {
				motor_id: 1,
				address: 116,
				data: 2000u32.to_le_bytes().as_slice(),
			},
			BulkWriteData {
				motor_id: 2,
				address: 102,
				data: 300u16.to_le_bytes().as_slice(),
			},
		])?;
		Ok(())
	}

	/// Ensure that `bulk_write` accepts a reference to a Vec and doesn't clone the data in the vector.
	///
	/// This is a compile test. It only tests that the test code compiles.
	#[allow(dead_code)]
	fn bulk_write_accepts_vec_ref_no_clone(
		bus: &mut Bus<Vec<u8>, Vec<u8>, serial2::SerialPort>,
	) -> Result<(), Box<dyn std::error::Error>> {
		/// Non-clonable wrapper around `&[u8]` to ensure `bulk_write` doesn't clone data from vec references.
		struct Data<'a> {
			data: &'a [u8],
		}

		impl AsRef<[u8]> for Data<'_> {
			fn as_ref(&self) -> &[u8] {
				self.data
			}
		}

		impl<'a> Data<'a> {
			fn new<const N: usize>(data: &'a [u8; N]) -> Self {
				Self { data: data.as_slice() }
			}
		}

		bus.bulk_write(&vec![
			BulkWriteData {
				motor_id: 1,
				address: 116,
				data: Data::new(&2000u32.to_le_bytes()),
			},
			BulkWriteData {
				motor_id: 2,
				address: 102,
				data: Data::new(&300u16.to_le_bytes()),
			},
		])?;
		Ok(())
	}
}
