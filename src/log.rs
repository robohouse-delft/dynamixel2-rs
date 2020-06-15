#[cfg(feature = "log")]
#[allow(unused)]
#[macro_use]
mod log {
	macro_rules! trace {
		($($args:tt)*) => { ::log::trace!($($args)*) }
	}

	macro_rules! debug {
		($($args:tt)*) => { ::log::debug!($($args)*) }
	}

	macro_rules! info {
		($($args:tt)*) => { ::log::info!($($args)*) }
	}

	macro_rules! warn {
		($($args:tt)*) => { ::log::warn!($($args)*) }
	}

	macro_rules! error {
		($($args:tt)*) => { ::log::error!($($args)*) }
	}
}

#[cfg(not(feature = "log"))]
#[allow(unused)]
#[macro_use]
mod log {
	macro_rules! trace {
		($($args:tt)*) => {}
	}

	macro_rules! debug {
		($($args:tt)*) => {}
	}

	macro_rules! info {
		($($args:tt)*) => {}
	}

	macro_rules! warn {
		($($args:tt)*) => {}
	}

	macro_rules! error {
		($($args:tt)*) => {}
	}
}
