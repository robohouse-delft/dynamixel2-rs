use serial::SerialPort;

use dynamixel2::instructions::Read;

fn main() {
	if do_main().is_err() {
		std::process::exit(1);
	}
}

fn print_usage() {
	eprintln!("usage: read TTY BAUD-RATE MOTOR-ID ADDRESS LENGTH");
}

fn do_main() -> Result<(), ()> {
	let mut args = std::env::args();

	let _ = args.next().unwrap();

	#[cfg(feature = "log")]
	{
		env_logger::from_env("RUST_LOG").filter_level(log::LevelFilter::Trace).init();
	}

	let tty = args.next().ok_or_else(print_usage)?;
	let baud_rate = args.next().ok_or_else(print_usage)?;
	let motor_id = args.next().ok_or_else(print_usage)?;
	let address = args.next().ok_or_else(print_usage)?;
	let length = args.next().ok_or_else(print_usage)?;

	let baud_rate : usize = baud_rate.parse().map_err(|_| eprintln!("invalid baud rate: {}", baud_rate))?;
	let motor_id  : u8    = motor_id.parse().map_err(|_| eprintln!("invalid motor ID: {}", motor_id))?;
	let address   : u16   = address.parse().map_err(|_| eprintln!("invalid register address: {}", address))?;
	let length    : u16   = length.parse().map_err(|_| eprintln!("invalid length: {}", length))?;

	let baud_rate = match baud_rate {
		110  => serial::Baud110,
		300  => serial::Baud300,
		600  => serial::Baud600,
		1200  => serial::Baud1200,
		2400  => serial::Baud2400,
		4800  => serial::Baud4800,
		9600  => serial::Baud9600,
		19200  => serial::Baud19200,
		38400  => serial::Baud38400,
		57600  => serial::Baud57600,
		115200 => serial::Baud115200,
		other  => serial::BaudOther(other),
	};

	let mut tty = serial::open(&tty).map_err(|e| eprintln!("failed to open serial port at {}: {}", tty, e))?;

	let config = serial::PortSettings {
		baud_rate,
		char_size: serial::Bits8,
		stop_bits: serial::Stop1,
		flow_control: serial::FlowNone,
		parity: serial::ParityNone,
	};

	eprintln!("configuring serial port with: {:#?}", config);
	tty.configure(&config).map_err(|e| eprintln!("failed to configure serial port: {}", e))?;

	let mut read_buffer = vec![0u8; length.into()];
	let mut request = Read::new(motor_id, address, &mut read_buffer);
	dynamixel2::write_request(&mut tty, &request)
		.map_err(|e| eprintln!("failed to send PING instruction: {}", e))?;
	let status = dynamixel2::read_response(&mut tty, &mut request)
		.map_err(|e| eprintln!("failed to read PING status: {:?}", e))?;

	println!("{:02X?}", read_buffer);
	Ok(())
}
