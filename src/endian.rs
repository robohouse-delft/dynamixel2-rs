pub fn write_u16_le(buffer: &mut [u8], value: u16) {
	buffer[0] = (value >> 0 & 0xFF) as u8;
	buffer[1] = (value >> 8 & 0xFF) as u8;
}

pub fn read_u16_le(buffer: &[u8]) -> u16 {
	buffer[0] as u16 | (buffer[1] as u16) << 8
}
