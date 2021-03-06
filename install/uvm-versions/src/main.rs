use uvm_cli;
use uvm_versions;
#[macro_use]
extern crate log;


use console::style;
use std::process;
use uvm_versions::VersionsOptions;

const USAGE: &str = "
uvm-versions - List available Unity versions to install.

Usage:
  uvm-versions [options] [<pattern>]
  uvm-versions (-h | --help)

Arguments:
  pattern           a regex pattern to filter the result

Options:
  -a, --all         list all available versions for the selected version types
  -f, --final       list final versions
  -b, --beta        list beta versions
  --alpha           list alpha versions
  -p, --patch       list patch versions
  -v, --verbose     print more output
  -d, --debug       print debug output
  --color WHEN      Coloring: auto, always, never [default: auto]
  -h, --help        show this help message and exit
";

fn main() -> std::io::Result<()> {
    let options: VersionsOptions = uvm_cli::get_options(USAGE)?;
    uvm_versions::UvmCommand::new()
        .exec(&options)
        .unwrap_or_else(|err| {
            let message = "Failure listing available versions";
            eprintln!("{}", style(message).red());
            info!("{}", &format!("{}", style(err).red()));
            process::exit(1);
        });
    Ok(())
}
