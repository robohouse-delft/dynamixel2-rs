/// Write a u8 to a buffer in little endian format.
///
/// Although endianness has no meaning for single-byte values,
/// having this function keeps the code consistent and easier to read.
pub fn write_u8_le(buffer: &mut [u8], value: u8) {
	buffer[0] = value;
}

/// Read a u8 from a buffer in little endian format.
///
/// Although endianness has no meaning for single-byte values,
/// having this function keeps the code consistent and easier to read.
pub fn read_u8_le(buffer: &[u8]) -> u8 {
	buffer[0]
}

/// Write a u16 to a buffer in little endian format.
pub fn write_u16_le(buffer: &mut [u8], value: u16) {
	buffer[0] = (value & 0xFF) as u8;
	buffer[1] = (value >> 8 & 0xFF) as u8;
}

/// Read a u16 in little endian format from a buffer.
pub fn read_u16_le(buffer: &[u8]) -> u16 {
	let low = buffer[0] as u16;
	let high = buffer[1] as u16;
	low | high << 8
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_write_u16_le() {
		let mut buffer = [0xFF; 4];
		write_u16_le(&mut buffer[0..], 0x0000);
		assert!(buffer == [0x00, 0x00, 0xFF, 0xFF]);

		write_u16_le(&mut buffer[2..], 0x1234);
		assert!(buffer == [0x00, 0x00, 0x34, 0x12]);
	}

	#[test]
	fn test_read_u16_le() {
		assert!(read_u16_le(&[0x00, 0x00, 0x34, 0x12]) == 0);
		assert!(read_u16_le(&[0x34, 0x12]) == 0x1234);
	}
}
