use assert2::let_assert;
use dynamixel2::Client;

static SERIAL_MUTEX: std::sync::LazyLock<std::sync::Mutex<()>> = std::sync::LazyLock::new(std::sync::Mutex::default);
const DEFAULT_SERIAL_PATH: &str = "/dev/ttyUSB0";
const DEFAULT_BAUD: u32 = 56_700;
const DEFAULT_IDS: &[u8] = &[1, 2];
pub fn run(test: impl FnOnce(&[u8], Client<serial2::SerialPort>)) {
    // prevent multiple threads trying to use the serial port
    let _lock = SERIAL_MUTEX.lock();
    let path = std::env::var("SERIAL_PATH").unwrap_or(DEFAULT_SERIAL_PATH.to_string());
    let baud = std::env::var("SERIAL_BAUD")
        .map(|s| {
            let_assert!(Ok(s) = s.parse(), "unable to parse SERIAL_BAUD {} into u32", s);
            s
        })
        .unwrap_or(DEFAULT_BAUD);
    let ids = std::env::var("DEVICE_IDS")
        .map(|ids| {
            ids.split(",")
                .map(|id| {
                    let_assert!(Ok(id) = id.parse(), "unable to parse DEVICE_IDS {} into Vec<u8>", ids);
                    id
                })
                .collect()
        })
        .unwrap_or(DEFAULT_IDS.to_vec());
    let_assert!(Ok(client) = Client::open(&path, baud), "unable to open serial port at {}", path);
    test(&ids, client)
}
