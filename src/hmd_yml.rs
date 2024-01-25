use std::{error::Error, fs, io};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::other_err;

pub(crate) const HMD_YML: &str = "hmd.yml";

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct HmdYml {
  pub(crate) ssh_address: String,
  pub(crate) project: String,
  #[serde(default)]
  pub(crate) artifacts: Vec<String>,
  #[serde(flatten)]
  pub(crate) stages: IndexMap<String, String>,
}

impl Default for HmdYml {
  fn default() -> Self {
    Self {
      project: String::default(),
      ssh_address: String::default(),
      artifacts: Vec::default(),
      stages: [
        ["lint", "cargo clippy"],
        ["test", "cargo test"],
        ["build", "cargo build --release"],
        ["run", "cargo run --release"],
      ]
      .into_iter()
      .map(|pair| pair.map(<_>::to_string).into())
      .collect(),
    }
  }
}

pub(crate) fn read() -> io::Result<HmdYml> {
  let yml = fs::read_to_string(HMD_YML).map_err(not_found_context)?;
  let hmd_yml: HmdYml =
    serde_yaml::from_str(&yml).map_err(parse_error_context)?;
  if hmd_yml.project.is_empty() {
    return Err(parse_error_context(empty_project_error()));
  }
  if hmd_yml.stages.is_empty() {
    return Err(parse_error_context(no_stages_error()));
  }
  Ok(hmd_yml)
}

pub(crate) fn write(
  project: &str,
  ssh_address: &str,
) -> io::Result<()> {
  let hmd_yml = HmdYml {
    project: project.to_owned(),
    ssh_address: ssh_address.to_owned(),
    ..read().unwrap_or_default()
  };
  let hmd_yml = serde_yaml::to_string(&hmd_yml).map_err(other_err)?;
  fs::write(HMD_YML, hmd_yml)?;
  Ok(())
}

fn not_found_context(err: io::Error) -> io::Error {
  match err.kind() {
    io::ErrorKind::NotFound => io::Error::new(
      io::ErrorKind::NotFound,
      format!("No hml.yml file. Try `hmd init`: {err}"),
    ),
    _ => err,
  }
}

fn parse_error_context(err: impl Error) -> io::Error {
  io::Error::new(
io::ErrorKind::InvalidData,
format!("Can't read hmd.yml file. Invalid format. Try `hmd init` to overwrite: {err}"),
  )
}

fn empty_project_error() -> io::Error {
  io::Error::new(
    io::ErrorKind::InvalidData,
    "Field `project` in hmd.yml can't be empty",
  )
}

fn no_stages_error() -> io::Error {
  io::Error::new(io::ErrorKind::InvalidData, "No stages in hmd.yml")
}
