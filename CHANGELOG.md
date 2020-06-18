0.1.2:
  * Fix `write_u32_le()` to actually write in little endian.

0.1.1:
  * Fix encoding of WriteU8 instruction parameters.

0.1.0:
  * Initial release.
  * Implemented instructions: ping, read, write, reboot.
  * Add function to write a single instruction.
  * Add function to read a single response.
  * Add function to execute unicast intstructions.
  * Add function to scan a bus for motors.
