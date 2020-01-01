# SerialCat

Serial port commandline interface for embedded development.

## Installation

```sh
$ git clone https://github.com/syuzuki/serialcat
$ cd serialcat
$ cargo install --path .
```

## Usage

```sh
$ sc [OPTIONS] <PORT>
```

You can see all options by putting `-h` option. (see below)

### Examples

```sh
$ # Basic usage in *nix systems with /dev/ttyACM0
$ sc /dev/ttyACM0
$ # View help
$ sc -h
$ # Run in baud rate 115200bps
$ sc -b 115200 /dev/ttyACM0
$ # Do not visualize control characters and invalid UTF-8 sequence (for pipeline)
$ sc -r /dev/ttyACM0
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
