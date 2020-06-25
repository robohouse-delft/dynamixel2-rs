# dynamixel2 [![docs][docs-badge]][docs] [![tests][tests-badge]][tests]
[docs]: https://docs.rs/dynamixel2/
[tests]: https://github.com/robohouse-delft/dynamixel2-rs/actions?query=workflow%3Atests
[docs-badge]: https://docs.rs/dynamixel2/badge.svg
[tests-badge]: https://github.com/robohouse-delft/dynamixel2-rs/workflows/tests/badge.svg

An implementation of the [Dynamixel Protocol 2.0].

[Dynamixel Protocol 2.0]: https://emanual.robotis.com/docs/en/dxl/protocol2/

This library aims to provide a easy to use but low level implementation of the Dynamixel Protocol 2.0.
That means it allows you to execute arbitrary commands with arbitrary parameters.

The library does not currently aim to provide an easy interface to the higher level functions of a servo,
such as moving it to a specific angle or at a specific speed.
Instead, you will have to write the appropriate values to the correct registers yourself.

The main interface is the `transfer_single` function,
which can be used to send an instruction to a servo and read a single reply.
The function can work with any `Read` + `Write` stream.

It is also also to use `write_instruction` followed by multiple `read_response` calls
to receive replies from multiple motors.

See the `instructions` module for available instructions.

## Instruction implementation status

The following instructions are currently implemented (PRs welcome!):

* [x] Ping
* [x] Read
* [x] Write
* [x] Reg Write
* [x] Action
* [ ] Factory Reset
* [x] Reboot
* [ ] Clear
* [ ] Sync Read
* [ ] Sync Write
* [ ] Bulk Read
* [ ] Bulk Write
* [x] Custom raw instructions

## Functionaility implementation status

The following planned functionality is currently implemented (PRs welcome!):
* [x] Write instruction messages.
* [x] Read response (status) messages.
* [x] Bit-stuffing and de-stuffing of messages.
* [x] Checksum calculation and verification.
* [x] Optional logging of all instructions and responses.
* [ ] Optional integration with [`serial`](https://docs.rs/serial).
* [x] Utility function to perform unicast instructions.
* [ ] Utility function to perform broadcast instructions.
* [x] Utility function to scan a bus for motors.

## Optional features

You can enable the `log` feature to have the library use `log::trace!()` to log all sent instructions and received replies.
