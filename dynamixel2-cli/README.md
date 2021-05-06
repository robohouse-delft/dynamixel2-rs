# dynamixel2-cli
Command line utility to communicate with Dynamixel protocol 2.0 motors.

```
USAGE:
    dynamixel2 [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -v, --verbose
            Print more verbose messages. Can be used multiple times

    -s, --serial-port <serial-port>
            The serial port to use [default: /dev/ttyUSB0]

    -b, --baud-rate <baud-rate>
            The baud rate for the serial port [default: 9600]

    -h, --help
            Prints help information

    -V, --version
            Prints version information


SUBCOMMANDS:
    ping                Ping a motor or scan the entire bus
    reboot              Reboot a motor
    read8               Read an 8-bit value from a motor
    read16              Read a 16-bit value from a motor
    read32              Read a 32-bit value from a motor
    write8              Write an 8-bit value to a motor
    write16             Write a 16-bit value to a motor
    write32             Write a 32-bit value to a motor
    shell-completion    Write shell completions to standard output or a file
    help                Prints this message or the help of the given subcommand(s)
```
