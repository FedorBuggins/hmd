use std::{fs, io};

use serde::{Deserialize, Serialize};

use crate::{other_err, HMD_ROOT};

pub(crate) const HMD_CONFIG_YML: &str = "~/.hmd/config.yml";

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct HmdConfigYml {
  pub(crate) ssh_address: String,
}

impl HmdConfigYml {
  pub(crate) fn new(ssh_address: String) -> Self {
    Self { ssh_address }
  }
}

pub(crate) fn read() -> io::Result<HmdConfigYml> {
  let home = std::env::var("HOME").map_err(other_err)?;
  let yml =
    fs::read_to_string(HMD_CONFIG_YML.replacen('~', &home, 1))?;
  let hmd_config_yml: HmdConfigYml = serde_yaml::from_str(&yml)
    .map_err(|err| {
      other_err(format!("Can't read hmd config: {err}"))
    })?;
  Ok(hmd_config_yml)
}

pub(crate) fn write(ssh_address: String) -> io::Result<()> {
  let hmd_config_yml = HmdConfigYml::new(ssh_address);
  let yml =
    serde_yaml::to_string(&hmd_config_yml).map_err(|err| {
      other_err(format!("Can't serialize hmd config: {err}"))
    })?;
  let home = std::env::var("HOME").map_err(other_err)?;
  fs::create_dir_all(HMD_ROOT.replacen('~', &home, 1))?;
  fs::write(HMD_CONFIG_YML.replacen('~', &home, 1), yml)?;
  Ok(())
}
