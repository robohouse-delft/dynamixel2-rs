use std::sync::{LazyLock, Mutex};
use assert2::{assert, let_assert};
use dynamixel2::instructions::BulkReadData;
use test_log::test;
use dynamixel2::Client;

mod common;

const DEVICE_IDS: &'static [u8] = &[1, 2];

static SERIAL_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);



#[cfg(feature = "integration_test")]
use serial2::SerialPort;
#[cfg(feature = "integration_test")]
fn run(_device_ids: &[u8], test: impl FnOnce(Client<serial2::SerialPort>)) {
	// prevent multiple threads trying to use the serial port
	let _lock = SERIAL_MUTEX.lock();
	let path = std::env::var("SERIAL_PATH").unwrap_or(String::from("/dev/ttyUSB0"));
	let baud = std::env::var("SERIAL_BAUD")
		.map(|s|
				 {
					 let_assert!(Ok(s) = s.parse(), "unable to parse SERIAL_BAUD {} into u32", s);
					 s
				 }
	).unwrap_or(56700);
	let_assert!(Ok(client) = Client::open(&path, baud), "unable to open serial port at {}", path);
	test(client)
}
#[cfg(not(feature = "integration_test"))]
fn run(device_ids: &[u8], test: impl FnOnce(Client<common::MockSerial>)) {
	common::setup_mock_client_device(device_ids, test);
}
#[test]
fn test_read() {
	run(DEVICE_IDS, |mut client| {
		let_assert!(Ok(response) = client.read::<u32>(DEVICE_IDS[0], 132));
		assert!(response.motor_id == 1);
	})
}

#[test]
fn test_read_bytes() {
	run(DEVICE_IDS, |mut client| {
		let_assert!(Ok(response) = client.read_bytes::<Vec<u8>>(DEVICE_IDS[0], 132, 4));
		assert!(response.motor_id == 1);
	})
}

#[test]
fn test_write() {
	run(DEVICE_IDS, |mut client| {
		let_assert!(Ok(response) = client.write(DEVICE_IDS[0], 65, &1u8));
		assert!(response.motor_id == 1);
		let _ = client.write(1, 65, &0u8);
	})
}

#[test]
fn test_reg_write() {
	run(DEVICE_IDS, |mut client| {
		let_assert!(Ok(response) = client.reg_write(DEVICE_IDS[0], 65, &1u8));
		assert!(response.motor_id == 1);
		let_assert!(Ok(response) = client.action(DEVICE_IDS[0]));
		assert!(response.motor_id == 1);
		let _ = client.write(DEVICE_IDS[0], 65, &0u8);
	})
}

#[test]
fn test_sync_read() {
	run(DEVICE_IDS, |mut client| {
		let response = client.sync_read::<u32>(DEVICE_IDS, 132).unwrap();
		for (r, id) in response.zip(DEVICE_IDS) {
			match r {
				Err(e) => panic!("id {id} {e}"),
				Ok(r) => {
					assert!(r.motor_id == *id)
				},
			}
		}
	})
}

#[test]
fn test_sync_read_bytes() {
	run(DEVICE_IDS, |mut client| {
		let response = client.sync_read_bytes::<Vec<u8>>(DEVICE_IDS, 132, 4).unwrap();
		for (r, id) in response.zip(DEVICE_IDS) {
			match r {
				Err(e) => panic!("id {id} {e}"),
				Ok(r) => {
					assert!(r.motor_id == *id)
				},
			}
		}
	})
}

#[test]
fn test_bulk_read_bytes() {
	run(DEVICE_IDS, |mut client| {
		let bulk_read_data: Vec<_> = DEVICE_IDS
			.iter()
			.map(|id| BulkReadData {
				motor_id: *id,
				address: 132,
				count: 4,
			})
			.collect();
		let response = client.bulk_read_bytes::<Vec<u8>>(&bulk_read_data).unwrap();
		for (r, id) in response.zip(DEVICE_IDS) {
			match r {
				Err(e) => panic!("id {id} {e}"),
				Ok(r) => {
					assert!(r.motor_id == *id)
				},
			}
		}
	})
}

#[test]
fn test_ping() {
	run(DEVICE_IDS, |mut client| {
		let response = client.scan().unwrap();
		assert!(response.into_iter().count() == DEVICE_IDS.len(), "missing motor ping");
	})
}
