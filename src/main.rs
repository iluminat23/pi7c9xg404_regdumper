use bitflags::bitflags;
use clap::Parser;
use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError, LinuxI2CMessage};

const DEFAULT_I2C_BUS: u8 = 1;
const DEFAULT_CHIP_ADDR: u16 = 0x38;
const PORT_SIZE: u16 = 0x200;

bitflags! {
    struct B0Cmd: u8 {
        const READ = 0b100;
    }

    struct B1PortSel: u8 {
        const PORT0 = 0b0;
        const PORT1 = 0b0;
        const PORT2 = 0b1;
        const PORT3 = 0b1;
    }

    struct B2PortSel: u8 {
        const PORT0 = 0b0 << 7;
        const PORT1 = 0b1 << 7;
        const PORT2 = 0b0 << 7;
        const PORT3 = 0b1 << 7;
    }

    struct B2ByteEnable: u8 {
        const BYTE4 = 0b1 << 2;
        const BYTE3 = 0b1 << 3;
        const BYTE2 = 0b1 << 4;
        const BYTE1 = 0b1 << 5;
        const ALL = 0b1111 << 2;
    }
}

#[derive(Debug, Clone, Copy)]
enum Port {
    Port0,
    Port1,
    Port2,
    Port3,
}

struct Pi7c9xg404 {
    i2c_dev: LinuxI2CDevice,
}

impl Pi7c9xg404 {
    pub fn init(i2c_dev: u8, i2c_addr: u16) -> Result<Pi7c9xg404, LinuxI2CError> {
        let dev = unsafe { LinuxI2CDevice::force_new(format!("/dev/i2c-{}", i2c_dev), i2c_addr) }?;
        Ok(Pi7c9xg404 { i2c_dev: dev })
    }

    pub fn read_reg(&mut self, port: Port, offset: u16) -> Result<[u8; 4], LinuxI2CError> {
        let mut write_data = [0; 0x4];

        write_data[0] = B0Cmd::READ.bits();

        write_data[1] = match port {
            Port::Port0 => B1PortSel::PORT0.bits(),
            Port::Port1 => B1PortSel::PORT1.bits(),
            Port::Port2 => B1PortSel::PORT2.bits(),
            Port::Port3 => B1PortSel::PORT3.bits(),
        };

        write_data[2] = match port {
            Port::Port0 => B2PortSel::PORT0.bits(),
            Port::Port1 => B2PortSel::PORT1.bits(),
            Port::Port2 => B2PortSel::PORT2.bits(),
            Port::Port3 => B2PortSel::PORT3.bits(),
        } | B2ByteEnable::ALL.bits()
            | (offset >> 10) as u8;

        write_data[3] = (offset >> 2) as u8;

        let mut read_data = [0; 0x4];
        let mut msg = [
            LinuxI2CMessage::write(&write_data),
            LinuxI2CMessage::read(&mut read_data),
        ];
        match self.i2c_dev.transfer(&mut msg) {
            Ok(_) => (),
            Err(e) => eprintln!("{e}"),
        }
        Ok(read_data)
    }

    pub fn print_port_regs(&mut self, port: Port) -> Result<(), LinuxI2CError> {
        println!("Port: {:?}", port);
        for reg in 0..(PORT_SIZE / 4) {
            let val =self.read_reg(port, reg * 4)?;
            print_reg(reg * 4, val);
        }
        Ok(())
    }
}

fn print_reg(reg_num: u16, val: [u8; 4]) {
    println!("{:#06x}: {:#04x}  {:#04x} {:#04x} {:#04x}", reg_num, val[0], val[1], val[2], val[3]);
}

/// Convert a string slice to an integer, the base is determine from the prefix.
///
/// The string may contain 0b (for binary), 0o (for octal), 0x (for hex) or no
/// prefix (for decimal) values.
/// # Examples
///
/// ```
/// assert_eq!(parse_prefixed_int("0xA"), Ok(10));
/// ```
fn parse_prefixed_int<T>(src: &str) -> Result<T, String>
where
    T: num::Unsigned + num::Num<FromStrRadixErr = std::num::ParseIntError>,
{
    let val = if src.starts_with("0b") {
        T::from_str_radix(&src[2..], 2)
    } else if src.starts_with("0o") {
        T::from_str_radix(&src[2..], 8)
    } else if src.starts_with("0x") {
        T::from_str_radix(&src[2..], 16)
    } else {
        T::from_str_radix(src, 10)
    };
    match val {
        Ok(val) => Ok(val),
        Err(e) => Err(format!("{e}")),
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, default_value_t = DEFAULT_I2C_BUS, value_parser = parse_prefixed_int::<u8>)]
    i2c_bus: u8,
    #[clap(short, long, default_value_t = DEFAULT_CHIP_ADDR, value_parser = parse_prefixed_int::<u16>)]
    chip_addr: u16,
}

fn main() {
    let cli = Cli::parse();

    println!("/dev/i2c-{}: {:#04x}", cli.i2c_bus, cli.chip_addr);

    let mut pi7c9xg404 = match Pi7c9xg404::init(cli.i2c_bus, cli.chip_addr) {
        Ok(pi7c9xg404) => pi7c9xg404,
        Err(e) => {
            eprintln!(
                "ERROR: Can't access device {:#04x}@i2c-{}: {e}",
                cli.chip_addr, cli.i2c_bus
            );
            std::process::exit(-1);
        }
    };

    pi7c9xg404.print_port_regs(Port::Port0).unwrap();
    pi7c9xg404.print_port_regs(Port::Port1).unwrap();
    pi7c9xg404.print_port_regs(Port::Port2).unwrap();
    pi7c9xg404.print_port_regs(Port::Port3).unwrap();
}
