use anyhow::Result;
use argh::FromArgs;

mod converter;

/// Device Faker configuration tool
#[derive(FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    /// Convert Magisk module ZIP to TOML configuration
    Convert(ConvertArgs),
    /// Convert a system.prop-style file to TOML configuration
    ConvertProps(ConvertPropsArgs),
    /// Read current device properties via getprop and export TOML configuration
    DumpDevice(DumpDeviceArgs),
}

/// Convert Magisk module ZIP to TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "convert")]
struct ConvertArgs {
    /// input ZIP file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Convert a system.prop-style file to TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "convert-props")]
struct ConvertPropsArgs {
    /// input system.prop or property text file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output TOML file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Read current device properties via getprop and export TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "dump-device")]
struct DumpDeviceArgs {
    /// output TOML file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    match cli.command {
        Command::Convert(args) => {
            converter::convert_zip_config(&args.input, &args.output)?;
        }
        Command::ConvertProps(args) => {
            converter::convert_props_config(&args.input, &args.output)?;
        }
        Command::DumpDevice(args) => {
            converter::dump_current_device_config(&args.output)?;
        }
    }

    Ok(())
}
