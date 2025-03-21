use crate::bus::StatusPacket;
use crate::Response;
use core::mem::MaybeUninit;

/// A fixed-size type that can be read or written over the bus.
pub trait Data: Sized {
	/// The size in bytes of the encoded value.
	const ENCODED_SIZE: u16;

	/// Encode the value into the given buffer.
	fn encode(&self, buffer: &mut [u8]) -> Result<(), crate::error::BufferTooSmallError>;

	/// Decode the value from the given buffer.
	fn decode(buffer: &[u8]) -> Result<Self, crate::InvalidMessage>;
}

pub(crate) fn decode_status_packet<T: Data, E>(status_packet: StatusPacket) -> Result<Response<T>, crate::error::ReadError<E>> {
	crate::error::MotorError::check(status_packet.error())?;
	Ok(Response {
		motor_id: status_packet.packet_id(),
		alert: status_packet.alert(),
		data: T::decode(status_packet.parameters())?,
	})
}

pub(crate) fn decode_status_packet_bytes<'a, T>(status_packet: StatusPacket<'a>) -> Result<Response<T>, crate::MotorError>
where
	T: From<&'a [u8]>,
{
	crate::error::MotorError::check(status_packet.error())?;
	Ok(Response {
		motor_id: status_packet.packet_id(),
		alert: status_packet.alert(),
		data: T::from(status_packet.parameters()),
	})
}

pub(crate) fn decode_status_packet_bytes_borrow<T>(status_packet: StatusPacket<'_>) -> Result<Response<&T>, crate::MotorError>
where
	T: ?Sized,
	[u8]: core::borrow::Borrow<T>,
{
	crate::error::MotorError::check(status_packet.error())?;
	Ok(Response {
		motor_id: status_packet.packet_id(),
		alert: status_packet.alert(),
		data: core::borrow::Borrow::borrow(status_packet.parameters()),
	})
}

macro_rules! impl_data_for_number {
	($type:ty) => {
		impl Data for $type {
			const ENCODED_SIZE: u16 = to_u16(core::mem::size_of::<Self>());

			fn encode(&self, buffer: &mut [u8]) -> Result<(), crate::error::BufferTooSmallError> {
				const N: usize = core::mem::size_of::<$type>();
				crate::error::BufferTooSmallError::check(N, buffer.len())?;
				buffer[..N].copy_from_slice(&self.to_le_bytes());
				Ok(())
			}

			fn decode(buffer: &[u8]) -> Result<Self, crate::error::InvalidMessage> {
				const N: usize = core::mem::size_of::<$type>();
				crate::error::InvalidParameterCount::check(buffer.len(), N)?;
				let value = Self::from_le_bytes(buffer[0..N].try_into().unwrap());
				Ok(value)
			}
		}
	};
}

impl_data_for_number!(u8);
impl_data_for_number!(u16);
impl_data_for_number!(u32);
impl_data_for_number!(u64);
impl_data_for_number!(u128);
impl_data_for_number!(i8);
impl_data_for_number!(i16);
impl_data_for_number!(i32);
impl_data_for_number!(i64);
impl_data_for_number!(i128);

impl<T: Data, const N: usize> Data for [T; N] {
	const ENCODED_SIZE: u16 = T::ENCODED_SIZE.checked_mul(to_u16(N)).unwrap();

	fn encode(&self, buffer: &mut [u8]) -> Result<(), crate::error::BufferTooSmallError> {
		let encoded_size = T::ENCODED_SIZE as usize;
		crate::BufferTooSmallError::check(encoded_size * N, buffer.len())?;
		for (i, value) in self.iter().enumerate() {
			value.encode(&mut buffer[i * encoded_size..][..encoded_size])?;
		}
		Ok(())
	}

	fn decode(buffer: &[u8]) -> Result<Self, crate::InvalidMessage> {
		let encoded_size = T::ENCODED_SIZE as usize;
		let mut output = ArrayInitializer::new();
		for i in 0..N {
			let value = T::decode(&buffer[i * encoded_size..][..encoded_size])?;
			// SAFETY: We loop over 0..N, so we can not call `push()` more than `N` times.
			unsafe {
				output.push(value);
			}
		}
		unsafe {
			// SAFETY: We looped over 0..N, so we called `push()` exactly `N` times.
			Ok(output.finish())
		}
	}
}

struct ArrayInitializer<T: Sized, const N: usize> {
	data: [MaybeUninit<T>; N],
	initialized: usize,
}

impl<T: Sized, const N: usize> ArrayInitializer<T, N> {
	pub fn new() -> Self {
		Self {
			data: [const { MaybeUninit::uninit() }; N],
			initialized: 0,
		}
	}

	/// Add an element to the array.
	///
	/// # Safety
	/// You must ensure this function is called at most `N` times.
	pub unsafe fn push(&mut self, value: T) {
		debug_assert!(self.initialized < N);
		// SAFETY: The caller must ensure `push` is called at most N times.
		let slot = unsafe { self.data.get_unchecked_mut(self.initialized) };
		slot.write(value);
		self.initialized += 1;
	}

	/// Finalize the array, turning it into [T; N].
	///
	/// # Safety
	/// You must ensure you called `push()` exactly `N` times before calling this function.
	pub unsafe fn finish(self) -> [T; N] {
		debug_assert!(self.initialized == N);
		// SAFETY: The caller must guarantee that `self.data` is fully initialized,
		// if they did, reading it as `[T; N]` is sound.
		core::ptr::read(self.data.as_ptr().cast())
	}
}

impl<T, const N: usize> Drop for ArrayInitializer<T, N> {
	fn drop(&mut self) {
		for elem in &mut self.data[..self.initialized] {
			// SAFETY: We track which values are initialized and only drop those.
			// And since we're in Drop::drop() here, we won't drop the same value multiple times.
			unsafe {
				elem.assume_init_drop();
			}
		}
		self.initialized = 0;
	}
}

const fn to_u16(input: usize) -> u16 {
	assert!(input <= u16::MAX as usize);
	input as u16
}
