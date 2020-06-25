v0.1.3:
  * Implement `reg_write` and `action` instructions.
  * Implement `factory_reset` instruction.
  * Implement `clear` instruction.
  * Implement `sync_read` instruction.
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
