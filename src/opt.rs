//! Command line parser

use anyhow::Result;
use structopt::StructOpt;
use tokio_serial as serial;

fn data_bits_from_str(s: &str) -> Result<serial::DataBits> {
    use serial::DataBits::*;
    match s {
        "5" => Ok(Five),
        "6" => Ok(Six),
        "7" => Ok(Seven),
        "8" => Ok(Eight),
        _ => unreachable!(),
    }
}

fn parity_from_str(s: &str) -> Result<serial::Parity> {
    use serial::Parity::*;
    match s {
        "none" => Ok(None),
        "odd" => Ok(Odd),
        "even" => Ok(Even),
        _ => unreachable!(),
    }
}

fn stop_bits_from_str(s: &str) -> Result<serial::StopBits> {
    use serial::StopBits::*;
    match s {
        "1" => Ok(One),
        "2" => Ok(Two),
        _ => unreachable!(),
    }
}

fn flow_control_from_str(s: &str) -> Result<serial::FlowControl> {
    use serial::FlowControl::*;
    match s {
        "none" => Ok(None),
        "software" => Ok(Software),
        "hardware" => Ok(Hardware),
        _ => unreachable!(),
    }
}

/// Command line options.
///
/// [parse_args](fn.parse_args.html) parses command line arguments and returns this struct.
#[derive(Debug, Clone, PartialEq, Eq, StructOpt)]
pub struct Opt {
    #[structopt(help = "Serial port device", name = "port")]
    pub port: String,

    #[structopt(
        long,
        short,
        name = "BAUD",
        default_value = "9600",
        help = "Baud rate of serial port"
    )]
    pub baud_rate: u32,

    #[structopt(
        long,
        short,
        possible_values(&["5", "6", "7", "8"]),
        default_value = "8",
        help = "Data bits of serial port",
        name = "DATA_BITS",
        parse(try_from_str = data_bits_from_str)
    )]
    pub data_bits: serial::DataBits,

    #[structopt(
        long,
        short,
        possible_values(&["none", "odd", "even"]),
        default_value = "none",
        help = "Parity of serial port",
        name = "PARITY",
        parse(try_from_str = parity_from_str)
    )]
    pub parity: serial::Parity,

    #[structopt(
        long,
        short,
        possible_values(&["1", "2"]),
        default_value = "1",
        help = "Stop bits of serial port",
        name = "STOP_BITS",
        parse(try_from_str = stop_bits_from_str)
    )]
    pub stop_bits: serial::StopBits,

    #[structopt(
        long,
        short,
        possible_values(&["none", "software", "hardware"]),
        default_value = "none",
        help = "Flow control of serial port",
        name = "FLOW_CONTROL",
        parse(try_from_str = flow_control_from_str)
    )]
    pub flow_control: serial::FlowControl,

    #[structopt(
        long,
        short,
        help = "Do not visualize control characters and invalid UTF-8 sequence"
    )]
    pub raw: bool,

    #[structopt(
        long,
        short,
        help = "Quit when input EOF from stdin. Currently, do not quit if last character is not newline"
    )]
    pub escape_quit: bool,
}

/// Parse command line arguments.
///
/// This function parses command line arguments and returns [Opt](struct.Opt.html).
/// If command line arguments are help, version or invalid sequence, this function prints messages
/// and exits process immediately.
pub fn parse_args() -> Opt {
    Opt::from_args()
}

#[cfg(test)]
mod tests {
    use super::*;

    use serial::{DataBits, FlowControl, Parity, StopBits};

    #[test]
    fn args() {
        let name = "sc";
        let default_port = "/dev/ttyACM0";
        let default = Opt {
            port: default_port.to_owned(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            raw: false,
            escape_quit: false,
        };

        // default
        let args = Opt::from_iter_safe(&[&name, &default_port]).unwrap();
        assert_eq!(args, default);

        // other port name
        let args = Opt::from_iter_safe(&[&name, "/dev/ttyACM1"]).unwrap();
        assert_eq!(
            args,
            Opt {
                port: "/dev/ttyACM1".to_owned(),
                ..default.clone()
            }
        );

        // baud rate
        let args = Opt::from_iter_safe(&[&name, "-b", "115200", &default_port]).unwrap();
        assert_eq!(
            args,
            Opt {
                baud_rate: 115_200,
                ..default.clone()
            }
        );

        // data bits
        for (arg, enm) in &[
            ("5", DataBits::Five),
            ("6", DataBits::Six),
            ("7", DataBits::Seven),
            ("8", DataBits::Eight),
        ] {
            let args = Opt::from_iter_safe(&[&name, "-d", arg, &default_port]).unwrap();
            assert_eq!(
                args,
                Opt {
                    data_bits: *enm,
                    ..default.clone()
                }
            );
        }
        Opt::from_iter_safe(&[&name, "-d", "4", &default_port]).unwrap_err();
        Opt::from_iter_safe(&[&name, "-d", "9", &default_port]).unwrap_err();

        // parity
        for (arg, enm) in &[
            ("none", Parity::None),
            ("odd", Parity::Odd),
            ("even", Parity::Even),
        ] {
            let args = Opt::from_iter_safe(&[&name, "-p", arg, &default_port]).unwrap();
            assert_eq!(
                args,
                Opt {
                    parity: *enm,
                    ..default.clone()
                }
            );
        }
        Opt::from_iter_safe(&[&name, "-p", "crc", &default_port]).unwrap_err();

        // stop bits
        for (arg, enm) in &[("1", StopBits::One), ("2", StopBits::Two)] {
            let args = Opt::from_iter_safe(&[&name, "-s", arg, &default_port]).unwrap();
            assert_eq!(
                args,
                Opt {
                    stop_bits: *enm,
                    ..default.clone()
                }
            );
        }
        Opt::from_iter_safe(&[&name, "-s", "0", &default_port]).unwrap_err();
        Opt::from_iter_safe(&[&name, "-s", "3", &default_port]).unwrap_err();

        // flow control
        for (arg, enm) in &[
            ("none", FlowControl::None),
            ("software", FlowControl::Software),
            ("hardware", FlowControl::Hardware),
        ] {
            let args = Opt::from_iter_safe(&[&name, "-f", arg, &default_port]).unwrap();
            assert_eq!(
                args,
                Opt {
                    flow_control: *enm,
                    ..default.clone()
                }
            );
        }
        Opt::from_iter_safe(&[&name, "-f", "rts", &default_port]).unwrap_err();

        // raw
        let args = Opt::from_iter_safe(&[&name, "-r", &default_port]).unwrap();
        assert_eq!(
            args,
            Opt {
                raw: true,
                ..default.clone()
            }
        );

        // escape quit
        let args = Opt::from_iter_safe(&[&name, "-e", &default_port]).unwrap();
        assert_eq!(
            args,
            Opt {
                escape_quit: true,
                ..default.clone()
            }
        );
    }
}
