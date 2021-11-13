main:
  * Switch to `serial2` for serial communication.
  * Remove `Bus::with_buffer_sizes()` constructor.

v0.2.3:
  * Clear the read buffer when sending an instruction.
  * Add trace log for discarded partial messages after a timeout.

v0.2.2:
  * Add debug and trace logs for skipped noise bytes.

v0.2.1:
  * Change error type of sync write functions to reflect lack of read phase.
  * Add support for the bulk read and write instructions.

v0.2.0:
  * Ignore noise before message headers.
  * Rewrite API to a `Bus` struct with functions for the instructions.

v0.1.4:
  * Fix visibility of `WriteData` struct for `SyncWriteU*`.

v0.1.3:
  * Implement the `reg_write` and `action` instructions.
  * Implement the `factory_reset` instruction.
  * Implement the `clear` instruction.
  * Implement the `sync_read` and `sync_write` instructions.
  * Implement custom raw instructions.
  * Include correct license file.

v0.1.2:
  * Fix `write_u32_le()` to actually write in little endian.

v0.1.1:
  * Fix encoding of `WriteU8` instruction parameters.

v0.1.0:
  * Initial release.
  * Implemented instructions: `ping`, `read`, `write`, `reboot`.
  * Add function to write a single instruction.
  * Add function to read a single response.
  * Add function to execute unicast intstructions.
  * Add function to scan a bus for motors.
