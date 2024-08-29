pub fn init(root_module: &str, verbosity: i8) {
	use std::io::Write;

	let log_level = match verbosity {
		0 => log::LevelFilter::Info,
		1 => log::LevelFilter::Debug,
		_ => log::LevelFilter::Trace,
	};

	env_logger::Builder::new()
		.format(|buffer, record: &log::Record| {
			use env_logger::fmt::style;

			let mut prefix_style = style::Style::new();
			let prefix;

			match record.level() {
				log::Level::Trace => {
					prefix = "Trace: ";
				},
				log::Level::Debug => {
					prefix = "";
				},
				log::Level::Info => {
					prefix = "";
				},
				log::Level::Warn => {
					prefix = "Warning: ";
					prefix_style = prefix_style.fg_color(Some(style::AnsiColor::Yellow.into())).bold();
				},
				log::Level::Error => {
					prefix = "Error: ";
					prefix_style = prefix_style.fg_color(Some(style::AnsiColor::Red.into())).bold();
				},
			};

			writeln!(buffer, "{prefix_style}{prefix}{prefix_style:#} {}", record.args(),)
		})
		.filter_level(log::LevelFilter::Warn)
		.filter_module(root_module, log_level)
		.filter_module("dynamixel2", log_level)
		.init();
}
