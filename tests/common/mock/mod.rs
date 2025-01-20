use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use dynamixel2::{Client, Device};
mod mock_device;
mod mock_serial_port;

use mock_device::MockDevice;
use mock_serial_port::MockSerial;
pub fn new_client_device(device_ids: &[u8]) -> Result<(Client<MockSerial>, Vec<MockDevice>), std::io::Error> {
    let mut client_serial = MockSerial::new("Client");
    let device_ports: Vec<_> = device_ids.iter().map(|id| {
        let mut serial = MockSerial::new(&format!("device {id}"));
        client_serial.add_device(serial.read_buffer.clone());
        serial.add_device(client_serial.read_buffer.clone());
        (id, serial)

    }).collect();
    let device_ports_copy = device_ports.clone();
    let devices = device_ports.into_iter().map(|(id, mut d)| {
        // to each MockSerial, connect the read buffers from the other MockSerials
        device_ports_copy.iter().filter(|(other_id, _)| id != *other_id ).cloned().for_each(|(_, other)| {
            d.add_device(other.read_buffer);
        });
        let device = Device::new(d).unwrap();
        MockDevice::new(*id, device)
    }).collect();
    let client = Client::new(client_serial)?;

    Ok((client, devices))
}


pub fn run_mock<F>(test: F) where F: FnOnce(&[u8], Client<MockSerial>) {
    let device_ids = &[1, 2];
    let kill_device = Arc::new(AtomicBool::new(false));
    let (bus, devices) = new_client_device(device_ids).unwrap();
    let device_t = devices.into_iter().map(|d| d.run(kill_device.clone())).collect::<Vec<_>>();
    test(device_ids, bus);
    kill_device.store(true, Relaxed);
    device_t.into_iter().for_each(|d| d.join().unwrap());
}
