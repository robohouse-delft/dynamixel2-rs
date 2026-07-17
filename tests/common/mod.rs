#![cfg(feature = "std")]

pub mod mock;
// The real-hardware harness uses the synchronous `serial2` backend directly.
#[cfg(feature = "serial2")]
pub mod real;

#[cfg(not(feature = "integration-tests"))]
pub use mock::run_mock as run;

#[cfg(feature = "integration-tests")]
pub use real::run;
