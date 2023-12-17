v0.7.0 - 2023-12-17:
  * Pass `Response<&[u8]>` to read callbacks instead of `Response<Vec<u8>>`.

v0.6.1 - 2023-12-16:
  * Add `StatusPacket::error_number()`.
  * Add `MotorError::error_number()`.
  * Add `MotorError::alert()`.
  * Fix documentation fo `StatusPacket::alert()`.

v0.6.0 - 2023-12-16:
  * Fix amount of data read in `sync_read_u16` and `sync_read_u16_cb`.
  * Do not return `Err()` when the `alert` bit is set in a status packet from a motor.
  * Report the `alert` bit in the returned values from commands in a new `Response` struct.
  * Pass original `BulkReadData` command to user callback in `bulk_read_cb()`.

v0.5.1 - 2023-12-07:
  * Parse all status messages when more than on has been read in a single `read()`.

v0.5.0 - 2023-12-02:
  * Update `serial2` to `v0.2`.

v0.4.2 - 2023-12-02:
  * Remove unused generic parameter from `sync_read_*` functions.

v0.4.1 - 2022-12-12:
  * Fix the instruction ID used by the bulk read/write commands.

v0.4.0 - 2022-12-12:
  * Use `Borrow` trait instead of `AsRef` in `Bus::bulk_write()`.

v0.3.1:
  * Update documentation.

v0.3.0:
  * Switch to `serial2` for serial communication.
  * Remove `Bus::with_buffer_sizes()` constructor.
  * Discard input buffer right before writing instructions.

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
