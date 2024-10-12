mod mock_transport;

use crate::mock_transport::MockSerialPort;
use dynamixel2::{Bus, Device, Instructions, ReadError, TransferError, Transport};
use log::{info, trace};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use test_log::test;

type ReadBuffer = Vec<u8>;
type WriteBuffer = Vec<u8>;
type T = MockSerialPort;
fn setup_bus() -> (Bus<ReadBuffer, WriteBuffer, T>, Device<ReadBuffer, WriteBuffer, T>) {
	let transport = MockSerialPort::new(56700);
	let device_transport = transport.device_port();
	(
		Bus::with_buffers(transport, vec![0; 1024], vec![0; 1024]).unwrap(),
		Device::with_buffers(device_transport, vec![0; 1024], vec![0; 1024]).unwrap(),
	)
}

const DEVICE_ID: u8 = 1;

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
#[test]
fn test_packet_response() {
	info!("Testing packet response");
	trace!("Setting up bus and device");
	let kill_device = Arc::new(AtomicBool::new(false));
	let (mut bus, mut device) = setup_bus();
	let bus_t = thread::spawn(move || {
		bus.write_u8(1, 5, 1)?;
		let res = bus.read_u8(1, 5)?;
		assert_eq!(res.data, 1);
		Ok::<(), TransferError<<MockSerialPort as Transport>::Error>>(())
	});
	let device_t = thread::spawn({
		let kill_device = kill_device.clone();
		move || {
			let mut control_table = ControlTable::new(10);
			while !kill_device.load(Relaxed) {
				let res = device.read(Duration::from_secs(1));
				let packet = match res {
					Ok(p) => p,
					Err(ReadError::Io(e)) if T::is_timeout_error(&e) => continue,
					Err(e) => {
						return Err(e.into());
					},
				};
				let id = packet.id;
				if id != DEVICE_ID {
					continue;
				}
				match packet.instruction {
					Instructions::Ping => {},
					Instructions::Read { address, length } => {
						if let Some(data) = control_table.read(address, length) {
							device.write_status(DEVICE_ID, 0, length as usize, |buffer| {
								buffer.copy_from_slice(data);
							})?;
						} else {
							device.write_status_error(DEVICE_ID, 0x07)?;
						}
					},
					Instructions::Write { address, parameters } => {
						if control_table.write(address, parameters) {
							device.write_status_ok(DEVICE_ID)?;
						} else {
							device.write_status_error(DEVICE_ID, 0x07)?;
						}
					},
					i => todo!("impl {:?}", i),
				}
			}

			Ok::<(), TransferError<<MockSerialPort as Transport>::Error>>(())
		}
	});
	bus_t.join().unwrap().unwrap();
	kill_device.store(true, Relaxed);
	device_t.join().unwrap().unwrap();
}
