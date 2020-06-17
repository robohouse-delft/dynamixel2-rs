//! Byte-stuffing and de-stuffing.

pub const PATTERN: [u8; 4] = [0xFF, 0xFF, 0xFD, 0xFD];

/// Remove bit-stuffing in-place.
///
/// All patterns of `[0xFF, 0xFF, 0xFD, 0xFD]` will be replaced with `[0xFF, 0xFF, 0xFD]`.
#[must_use]
pub fn unstuff_inplace(data: &mut [u8]) -> usize {
	let mut deleted = 0;
	let mut state = 0;

	for i in 0..data.len() {
		if data[i] == PATTERN[state] {
			state += 1;
		} else {
			state = 0;
		}
		if state == 4 {
			state = 0;
			deleted += 1;
		} else if deleted > 0 {
			data[i - deleted] = data[i]
		}
	}

	data.len() - deleted
}

/// Calculate the maximum required size for stuffing arbitrary data of a certain length.
pub fn maximum_stuffed_len(unstuffed_length: usize) -> usize {
	unstuffed_length / 3 * 4 + unstuffed_length % 3
}

/// Calculate the amount of stuffing bytes required for specific data.
pub fn stuffing_required(data: &[u8]) -> usize {
	let mut state = 0;
	let mut count = 0;
	for &byte in data {
		if byte == PATTERN[state] {
			state += 1;
		} else {
			state = 0;
		}
		if state == 3 {
			state = 0;
			count += 1;
		}
	}

	count
}

/// Perform byte-stuffing in-place.
///
/// The actual length of the unstuffed data must be passed in through the `len` parameter.
///
/// The full buffer must be large enough to hold the stuffed data,
/// or an error is returned.
pub fn stuff_inplace(buffer: &mut [u8], len: usize) -> Result<usize, ()> {
	let stuffing_required = stuffing_required(&buffer[..len]);
	if stuffing_required == 0 {
		return Ok(len);
	}

	let mut read = 0;
	let mut stuffing_applied = 0;

	if buffer.len() < len + stuffing_required {
		return Err(());
	}

	while read < len && stuffing_applied < stuffing_required {
		let read_pos = len - read - 1;
		let write_pos = read_pos + (stuffing_required - stuffing_applied);
		if read_pos >= 2 && buffer[read_pos - 2..][..3] == PATTERN[..3] {
			buffer[write_pos - 3..][..4].copy_from_slice(&PATTERN);
			read += 3;
			stuffing_applied += 1;
		} else {
			buffer[write_pos] = buffer[read_pos];
			read += 1;
		}
	}

	debug_assert_eq!(stuffing_applied, stuffing_required);
	Ok(len + stuffing_required)
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	fn unstuff(mut data: Vec<u8>) -> Vec<u8> {
		let new_size = unstuff_inplace(&mut data);
		data.resize(new_size, 0);
		data
	}

	fn stuff(mut data: Vec<u8>) -> Result<Vec<u8>, ()> {
		let used = data.len();
		data.resize(maximum_stuffed_len(used), 0);
		let new_size = stuff_inplace(&mut data, used)?;
		data.resize(new_size, 0);
		Ok(data)
	}

	#[test]
	fn test_unstuff() {
		assert!(unstuff(vec![0, 0, 0]) == [0, 0, 0]);
		assert!(unstuff(vec![0xFF, 0xFF, 0xFD, 0x00]) == [0xFF, 0xFF, 0xFD, 0x00]);
		assert!(unstuff(vec![0xFF, 0xFF, 0xFD, 0xFD]) == [0xFF, 0xFF, 0xFD]);
		assert!(unstuff(vec![0xFF, 0xFF, 0x01, 0xFD, 0xFD]) == [0xFF, 0xFF, 0x01, 0xFD, 0xFD]);
		assert!(
			unstuff(vec![0x00, 0xFF, 0xFF, 0xFD, 0xFD, 0x01, 0xFF, 0xFF, 0xFD, 0xFD, 0x02, 0x03])
				== [0x00, 0xFF, 0xFF, 0xFD, 0x01, 0xFF, 0xFF, 0xFD, 0x02, 0x03]
		);
	}

	#[test]
	fn test_stuff() {
		assert!(stuff(vec![0, 0, 0]).unwrap() == [0, 0, 0]);
		assert!(stuff(vec![0xFF, 0xFF, 0xFD, 0x00]).unwrap() == [0xFF, 0xFF, 0xFD, 0xFD, 0x00]);
		assert!(stuff(vec![0xFF, 0xFF, 0xFD, 0xFD]).unwrap() == [0xFF, 0xFF, 0xFD, 0xFD, 0xFD]);
		assert!(stuff(vec![0xFF, 0xFF, 0x01, 0xFD]).unwrap() == [0xFF, 0xFF, 0x01, 0xFD]);
		assert!(
			stuff(vec![0x00, 0xFF, 0xFF, 0xFD, 0x01, 0xFF, 0xFF, 0xFD, 0x02, 0x03]).unwrap()
				== [0x00, 0xFF, 0xFF, 0xFD, 0xFD, 0x01, 0xFF, 0xFF, 0xFD, 0xFD, 0x02, 0x03]
		);
		assert!(stuff(vec![0xFF, 0xFF, 0xFD, 0xFF, 0xFF, 0xFFD]).unwrap() == [0xFF, 0xFF, 0xFD, 0xFD, 0xFF, 0xFF, 0xFD, 0xFD]);
		assert!(stuff(vec![0xFF, 0xFF, 0xFD, 0x00, 0xFF, 0xFF, 0xFFD]).unwrap() == [0xFF, 0xFF, 0xFD, 0xFD, 0x00, 0xFF, 0xFF, 0xFD, 0xFD]);
	}
}
