use assert2::{assert, let_assert};
use dynamixel2::{Client, Device, Instructions, ReadError, SerialPort};
use log::{info, trace};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::time::Duration;
use test_log::test;

mod mock_serial_port;
use mock_serial_port::MockSerialPort;

fn setup_bus() -> (Client<MockSerialPort>, Device<MockSerialPort>) {
	let serial_port = MockSerialPort::new(56700);
	let_assert!(Ok(device) = Device::new(serial_port.device_port()));
	let_assert!(Ok(client) = Client::new(serial_port));
	(client, device)
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
		assert!(let Ok(_) = bus.write_u8(1, 5, 1));
		let_assert!(Ok(response) =  bus.read_u8(1, 5));
		assert!(response.data == 1);
	});
	let device_t = thread::spawn({
		let kill_device = kill_device.clone();
		move || {
			let mut control_table = ControlTable::new(10);
			while !kill_device.load(Relaxed) {
				let packet = device.read(Duration::from_millis(50));
				let packet = match packet {
					Err(ReadError::Io(e)) if MockSerialPort::is_timeout_error(&e) => continue,
					x => x,
				};
				let_assert!(Ok(packet) = packet);
				let id = packet.id;
				if id != DEVICE_ID {
					continue;
				}
				match packet.instruction {
					Instructions::Ping => {},
					Instructions::Read { address, length } => {
						if let Some(data) = control_table.read(address, length) {
							assert!(let Ok(()) = device.write_status(DEVICE_ID, 0, length as usize, |buffer| {
								buffer.copy_from_slice(data);
							}));
						} else {
							assert!(let Ok(()) = device.write_status_error(DEVICE_ID, 0x07));
						}
					},
					Instructions::Write { address, parameters } => {
						if control_table.write(address, parameters) {
							let_assert!(Ok(()) = device.write_status_ok(DEVICE_ID));
						} else {
							let_assert!(Ok(()) = device.write_status_error(DEVICE_ID, 0x07));
						}
					},
					i => todo!("impl {:?}", i),
				}
			}
		}
	});
	bus_t.join().unwrap();
	kill_device.store(true, Relaxed);
	device_t.join().unwrap();
}
