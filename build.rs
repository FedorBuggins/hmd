use std::{env, io};

use clap::CommandFactory;
use clap_complete::{generate_to, shells::Bash};

include!("src/cli.rs");

fn main() -> io::Result<()> {
  let outdir = env::var("OUT_DIR").map_err(io::Error::other)?;
  let mut cmd = Cli::command();
  let bin_name = cmd.get_name().to_string();
  let path = generate_to(Bash, &mut cmd, bin_name, outdir)?;
  println!("cargo:warning=completion file is generated: {path:?}");
  Ok(())
}
