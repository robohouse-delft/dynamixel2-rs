pub fn write_u16_le(buffer: &mut [u8], value: u16) {
	buffer[0] = (value & 0xFF) as u8;
	buffer[1] = (value >> 8 & 0xFF) as u8;
}

pub fn read_u16_le(buffer: &[u8]) -> u16 {
	let low = buffer[0] as u16;
	let high = buffer[1] as u16;
	low | high << 8
}

pub fn write_u32_le(buffer: &mut [u8], value: u32) {
	write_u16_le(&mut buffer[0..2], (value >> 16 & 0xFFFF) as u16);
	write_u16_le(&mut buffer[2..4], (value & 0xFFFF) as u16);
}

pub fn read_u32_le(buffer: &[u8]) -> u32 {
	let low = read_u16_le(&buffer[0..2]) as u32;
	let high = read_u16_le(&buffer[2..4]) as u32;
	low | high << 16
}
