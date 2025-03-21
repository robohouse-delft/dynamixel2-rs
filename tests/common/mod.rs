pub mod mock;
pub mod real;

#[cfg(not(feature = "integration-tests"))]
pub use mock::run_mock as run;

#[cfg(feature = "integration-tests")]
pub use real::run;
