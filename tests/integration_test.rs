use assert2::{assert, let_assert};
use dynamixel2::instructions::BulkReadData;
use dynamixel2::Client;
use test_log::test;

pub mod common;

const DEVICE_IDS: &'static [u8] = &[1, 2];

#[cfg(feature = "integration_test")]
static SERIAL_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> = std::sync::LazyLock::new(std::sync::Mutex::default);

#[cfg(feature = "integration_test")]
fn run(test: impl FnOnce(&[u8], Client<serial2::SerialPort>)) {
	// prevent multiple threads trying to use the serial port
	let _lock = SERIAL_MUTEX.lock();
	let path = std::env::var("SERIAL_PATH").unwrap_or(String::from("/dev/ttyUSB0"));
	let baud = std::env::var("SERIAL_BAUD")
		.map(|s| {
			let_assert!(Ok(s) = s.parse(), "unable to parse SERIAL_BAUD {} into u32", s);
			s
		})
		.unwrap_or(56700);
	let_assert!(Ok(client) = Client::open(&path, baud), "unable to open serial port at {}", path);
	let ids = std::env::var("DEVICE_IDS")
		.map(|ids| {
			ids.split(",")
				.map(|id| {
					let_assert!(Ok(id) = id.parse(), "unable to parse DEVICE_IDS {} into Vec<u8>", ids);
					id
				})
				.collect()
		})
		.unwrap_or(DEVICE_IDS.to_vec());
	test(&ids, client)
}
#[cfg(not(feature = "integration_test"))]
fn run(test: impl FnOnce(&[u8], Client<common::MockSerial>)) {
	common::setup_mock_client_device(DEVICE_IDS, test);
}
#[test]
fn test_read() {
	run(|ids, mut client| {
		let_assert!(Ok(response) = client.read::<u32>(ids[0], 132));
		assert!(response.motor_id == ids[0]);
	})
}

#[test]
fn test_read_bytes() {
	run(|ids, mut client| {
		let_assert!(Ok(response) = client.read_bytes::<Vec<u8>>(ids[0], 132, 4));
		assert!(response.motor_id == ids[0]);
	})
}

#[test]
fn test_write() {
	run(|ids, mut client| {
		let_assert!(Ok(response) = client.write(ids[0], 65, &1u8));
		assert!(response.motor_id == ids[0]);
		let _ = client.write(ids[0], 65, &0u8);
	})
}

#[test]
fn test_reg_write() {
	run(|ids, mut client| {
		let_assert!(Ok(response) = client.reg_write(ids[0], 65, &1u8));
		assert!(response.motor_id == ids[0]);
		let_assert!(Ok(response) = client.action(ids[0]));
		assert!(response.motor_id == ids[0]);
		let _ = client.write(ids[0], 65, &0u8);
	})
}

#[test]
#[cfg_attr(not(feature = "integration_test"), should_panic)] // test panics as Mock Serial doesn't currently support sync_read
fn test_sync_read() {
	run(|ids, mut client| {
		let response = client.sync_read::<u32>(ids, 132).unwrap();
		for (r, id) in response.zip(ids) {
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
#[cfg_attr(not(feature = "integration_test"), should_panic)] // test panics as Mock Serial doesn't currently support sync_read
fn test_sync_read_bytes() {
	run(|ids, mut client| {
		let response = client.sync_read_bytes::<Vec<u8>>(ids, 132, 4).unwrap();
		for (r, id) in response.zip(ids) {
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
#[cfg_attr(not(feature = "integration_test"), should_panic)] // test panics as Mock Serial doesn't currently support bulk_read
fn test_bulk_read_bytes() {
	run(|ids, mut client| {
		let bulk_read_data: Vec<_> = ids
			.iter()
			.map(|id| BulkReadData {
				motor_id: *id,
				address: 132,
				count: 4,
			})
			.collect();
		let response = client.bulk_read_bytes::<Vec<u8>>(&bulk_read_data).unwrap();
		for (r, id) in response.zip(ids) {
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
	run(|ids, mut client| {
		let response = client.scan().unwrap();
		assert!(response.into_iter().count() == ids.len(), "missing motor ping");
	})
}
