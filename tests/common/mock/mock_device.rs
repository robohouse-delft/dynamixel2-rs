use super::mock_serial_port::MockSerial;
use assert2::let_assert;
use dynamixel2::{Device, Instruction, Instructions, ReadError, SerialPort};
use log::{error, trace, warn};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ControlTable {
	data: Vec<u8>,
}

impl ControlTable {
	pub fn new(size: usize) -> Self {
		Self { data: vec![0; size] }
	}

	pub fn read(&self, address: u16, length: u16) -> Option<&[u8]> {
		let start = address as usize;
		let end = start + length as usize;
		if end > self.data.len() {
			return None;
		}
		trace!("Read {:?} from {:?} in control table", &self.data[start..end], address);
		Some(&self.data[start..end])
	}

	pub fn write(&mut self, address: u16, data: &[u8]) -> bool {
		let start = address as usize;
		let end = start + data.len();
		if end > self.data.len() {
			return false;
		}
		self.data[start..end].copy_from_slice(data);
		trace!("Wrote {:?} to {:?} in control table", &self.data[start..end], address);
		true
	}
}

pub struct MockDevice {
	id: u8,
	pub device: Device<MockSerial>,
	control_table: ControlTable,
	action_register: Option<(u16, Vec<u8>)>,
}

impl MockDevice {
	pub(crate) fn new(id: u8, device: Device<MockSerial>) -> Self {
		MockDevice {
			id,
			device,
			control_table: ControlTable::new(200),
			action_register: None,
		}
	}

	pub(crate) fn run(mut self, kill: Arc<AtomicBool>) -> JoinHandle<()> {
		thread::spawn(move || {
			while !kill.load(Relaxed) {
				let packet = self.device.read(Duration::from_millis(10));
				let packet = match packet {
					Err(ReadError::Io(e)) if MockSerial::is_timeout_error(&e) => continue,
					x => x,
				};
				let_assert!(Ok(packet) = packet);
				let id = packet.id;
				if id != self.id && id != 254 {
					continue;
				}
				match packet.instruction {
					Instructions::Ping => {
						// todo: this should wait for based on id for some amount of time
						let_assert!(
							Ok(_) = self.device.write_status(self.id, 0, 3, |buffer| {
								buffer[0] = 10;
								buffer[1] = 20;
								buffer[2] = 46;
								Ok(())
							})
						);
					},
					Instructions::Read { address, length } => {
						if let Some(data) = self.control_table.read(address, length) {
							let_assert!(
								Ok(()) = self.device.write_status(self.id, 0, length as usize, |buffer| {
									buffer.copy_from_slice(data);
									Ok(())
								})
							);
						} else {
							let_assert!(Ok(()) = self.device.write_status_error(self.id, 0x07));
						}
					},
					Instructions::Write { address, parameters } => {
						if self.control_table.write(address, parameters) {
							let_assert!(Ok(()) = self.device.write_status_ok(self.id));
						} else {
							let_assert!(Ok(()) = self.device.write_status_error(self.id, 0x07));
						}
					},
					Instructions::SyncRead { address, length, ids } => {
						if !ids.contains(&self.id) {
							continue;
						}
						let ids = ids.to_vec();
						self.handle_sync_bulk_read(address, length, ids);
					},
					Instructions::SyncWrite {
						address,
						length,
						parameters,
					} => {
						let Some(id_index) = parameters.iter().position(|id| id == &self.id) else {
							continue;
						};
						let parameters = &parameters[id_index + 1..id_index + 1 + length as usize];
						self.control_table.write(address, parameters);
					},
					Instructions::BulkRead { parameters } => {
						let ids: Vec<_> = parameters.iter().step_by(5).copied().collect();
						let Some(id_index) = ids.iter().position(|id| id == &self.id) else {
							continue;
						};
						let id_index = id_index * 5;
						let address = u16::from_le_bytes(parameters[id_index + 1..id_index + 3].try_into().unwrap());
						let length = u16::from_le_bytes(parameters[id_index + 3..id_index + 5].try_into().unwrap());
						self.handle_sync_bulk_read(address, length, ids)
					},
					Instructions::BulkWrite { parameters } => {
						let id_index = {
							let mut id_index = 0;
							loop {
								let id = parameters[id_index];
								if id == self.id {
									break id_index;
								}
								let length = u16::from_le_bytes(parameters[id_index + 3..id_index + 5].try_into().unwrap());
								id_index += length as usize
							}
						};
						let address = u16::from_le_bytes(parameters[id_index + 1..id_index + 3].try_into().unwrap());
						let length = u16::from_le_bytes(parameters[id_index + 3..id_index + 5].try_into().unwrap());

						let _ = self
							.control_table
							.write(address, &parameters[id_index + 5..id_index + 5 + length as usize]);
					},
					Instructions::RegWrite { address, parameters } => {
						let _ = self.action_register.insert((address, parameters.to_vec()));
						let_assert!(Ok(()) = self.device.write_status_ok(self.id));
					},
					Instructions::Action => {
						if let Some((address, parameters)) = self.action_register.take() {
							if self.control_table.write(address, &parameters) {
								let_assert!(Ok(()) = self.device.write_status_ok(self.id));
							} else {
								let_assert!(Ok(()) = self.device.write_status_error(self.id, 0x07));
							}
						} else {
							let_assert!(Ok(()) = self.device.write_status_error(self.id, 0x07));
						}
					},
					Instructions::FactoryReset(_) => todo!("handle FactoryReset"),
					Instructions::Reboot => todo!("handle Reboot"),
					Instructions::Clear(_) => todo!("handle Clear"),
					Instructions::StatusPacket { .. } => (),
					Instructions::Unknown { instruction, .. } => error!("Unknown instruction {:?}", instruction),
				}
			}
		})
	}

	fn handle_sync_bulk_read(&mut self, address: u16, length: u16, ids: Vec<u8>) {
		for next_id in ids.clone() {
			if next_id == self.id {
				if let Some(data) = self.control_table.read(address, length) {
					let_assert!(
						Ok(()) = self.device.write_status(self.id, 0, length as usize, |buffer| {
							buffer.copy_from_slice(data);
							Ok(())
						})
					);
				} else {
					warn!("Failed to read from control table {address}, {length}");
					let_assert!(Ok(()) = self.device.write_status_error(self.id, 0x07));
				}
				return;
			} else {
				loop {
					trace!("{} waiting for packet from {}", self.id, next_id);
					let packet = self.device.read(Duration::from_millis(10));
					match packet {
						Ok(Instruction {
							id,
							instruction: Instructions::StatusPacket { .. },
						}) if id == next_id => {
							break;
						},
						Err(ReadError::Io(e)) if MockSerial::is_timeout_error(&e) => continue,
						Ok(packet) => {
							error!("aborting sync/bulk read due to unexpected packet {packet:?}");
							return;
						},
						Err(e) => {
							error!("aborting sync/bulk read due to read error {e}");
							return;
						},
					}
				}
			}
		}
	}
}
