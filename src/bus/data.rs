use core::mem::MaybeUninit;

/// A type that can be read or written over the bus.
pub trait Data<'a>: Sized {
	/// Encode the value into the given buffer.
	///
	/// On success, returns the number of bytes written to the buffer.
	fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::error::BufferTooSmallError>;

	/// Decode the value from the given buffer.
	///
	/// On success, returns the parsed value and the amount of bytes that were parsed from the buffer.
	fn decode(buffer: &[u8]) -> Result<(Self, usize), crate::InvalidMessage>;

	/// Get the size of the value when encoded.
	fn encoded_size(&self) -> usize;
}

/// A fized-size type that can be read or written over the bus.
pub trait FixedSizedData<'a>: Data<'a> {
	/// The size in bytes of the encoded value.
	const ENCODED_SIZE: usize;
}

macro_rules! impl_data_for_number {
	($type:ty) => {
		impl Data<'_> for $type {
			fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::error::BufferTooSmallError> {
				const N: usize = core::mem::size_of::<$type>();
				crate::error::BufferTooSmallError::check(N, buffer.len())?;
				buffer[..N].copy_from_slice(&self.to_le_bytes());
				Ok(N)
			}

			fn decode(buffer: &[u8]) -> Result<(Self, usize), crate::error::InvalidMessage> {
				const N: usize = core::mem::size_of::<$type>();
				if buffer.len() < N {
					todo!();
				}
				let value = Self::from_le_bytes(buffer[0..N].try_into().unwrap());
				Ok((value, N))
			}

			fn encoded_size(&self) -> usize {
				core::mem::size_of::<Self>()
			}
		}

		impl FixedSizedData<'_> for $type {
			const ENCODED_SIZE: usize = core::mem::size_of::<Self>();
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

impl<'a, T: FixedSizedData<'a>, const N: usize> Data<'a> for [T; N] {
	fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::error::BufferTooSmallError> {
		crate::BufferTooSmallError::check(T::ENCODED_SIZE * N, buffer.len())?;
		for (i, value) in self.iter().enumerate() {
			let size = value.encode(&mut buffer[i * T::ENCODED_SIZE..][..T::ENCODED_SIZE])?;
			debug_assert!(size == T::ENCODED_SIZE);
		}
		Ok(T::ENCODED_SIZE * N)
	}

	fn decode(buffer: &[u8]) -> Result<(Self, usize), crate::InvalidMessage> {
		let mut output = ArrayInitializer::new();
		for i in 0..N {
			let (value, size) = T::decode(&buffer[i * T::ENCODED_SIZE..][..T::ENCODED_SIZE])?;
			debug_assert!(size == T::ENCODED_SIZE);
			// SAFETY: We loop over 0..N, so we can not call `push()` more than `N` times.
			unsafe {
				output.push(value);
			}
		}
		unsafe {
			// SAFETY: We looped over 0..N, so we called `push()` exactly `N` times.
			Ok((output.finish(), T::ENCODED_SIZE * N))
		}
	}

	fn encoded_size(&self) -> usize {
		let mut encoded_size = 0;
		for elem in self {
			encoded_size += elem.encoded_size();
		}
		encoded_size
	}
}

impl<'a, T: FixedSizedData<'a>, const N: usize> FixedSizedData<'a> for [T; N] {
	const ENCODED_SIZE: usize = T::ENCODED_SIZE * N;
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
		let slot = unsafe {
			self.data.get_unchecked_mut(self.initialized)
		};
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
