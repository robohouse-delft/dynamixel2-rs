pub mod mock;
pub mod real;

#[cfg(not(feature = "integration_test"))]
pub use mock::run_mock as run;

#[cfg(feature = "integration_test")]
pub use real::run;
