#[cfg(feature = "log")]
#[allow(unused)]
#[macro_use]
mod details {
	macro_rules! trace {
		($($args:tt)*) => {{
			::log::trace!($($args)*);
		}}
	}

	macro_rules! debug {
		($($args:tt)*) => {{
			::log::debug!($($args)*);
		}}
	}

	macro_rules! info {
		($($args:tt)*) => {{
			::log::info!($($args)*);
		}}
	}

	macro_rules! warn {
		($($args:tt)*) => {{
			::log::warn!($($args)*);
		}}
	}

	macro_rules! error {
		($($args:tt)*) => {{
			::log::error!($($args)*);
		}}
	}
}

#[cfg(not(feature = "log"))]
#[allow(unused)]
#[macro_use]
mod details {
	// These macros all pass the arguments to `format_args!()`
	// to trigger compilation failures even with the "log" feature disabled.

	macro_rules! trace {
		($($args:tt)*) => {{
			format_args!($($args)*);
		}};
	}

	macro_rules! debug {
		($($args:tt)*) => {{
			format_args!($($args)*);
		}}
	}

	macro_rules! info {
		($($args:tt)*) => {{
			format_args!($($args)*);
		}}
	}

	macro_rules! warn {
		($($args:tt)*) => {{
			format_args!($($args)*);
		}}
	}

	macro_rules! error {
		($($args:tt)*) => {{
			format_args!($($args)*);
		}}
	}
}
