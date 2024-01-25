//! ## TODO:
//!
//! - [x] List command
//! - [x] Default project from hmd.yml
//! - [x] Project option
//! - [x] Deploy dirty changes
//! - [x] Refactor to optional project
//! - [x] Global config for ssh address
//! - [x] Use global config ssh address if not provided
//! - [x] Option for ssh address
//! - [x] Upload pipeline script
//! - [x] Status command
//! - [x] Restart command
//! - [ ] Scope option
//! - [ ] Verbose option
//! - [ ] Pretty logs
//! - [ ] Publish crate
//! - [ ] Completions
//! - [ ] Man
//! - [ ] Status option for deploy
//! - [ ] Log option for deploy
//! - [ ] Split by modules
//! - [ ] Config path option
//!

#![warn(clippy::pedantic)]

use std::{
  env,
  error::Error,
  fs, io, ops,
  process::{Command as Cmd, ExitCode},
};

use clap::{Args, Parser, Subcommand};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

const HMD_ROOT: &str = "~/.hmd";
const HMD_CONFIG_YML: &str = "~/.hmd/config.yml";
const HMD_YML: &str = "hmd.yml";

const SCRIPT: &str = include_str!("../script.sh");

const PROJECT_NOT_PROVIDED: &str = "Project not provided";

/// Home Deploy Tool
///
/// Prerequisites:
/// git, ssh, scp, configured ssh server
///
/// Useful git, ssh, scp commands to deploy
/// your pet project in one line.
/// All stages are performed as a single process.
/// The last stage can launch the application.
/// It will be stopped the next time it is deployed
/// or after manually calling the stop command.
///
#[derive(Parser)]
#[clap(version)]
struct Cli {
  #[clap(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Create project folder at ssh server and init `hmd.yml`
  Init {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },

  /// Push HEAD to server and run pipeline
  #[clap(visible_alias = "d")]
  Deploy {
    /// Push work tree with staged and unstaged changes
    #[clap(long)]
    dirty: bool,
  },

  /// Stop pipeline
  Stop {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },

  /// Restart pipeline
  Restart {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },

  /// Show pipeline status
  #[clap(visible_alias = "s")]
  Status {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },

  /// Stream pipeline log
  #[clap(visible_alias = "l")]
  Log {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },

  /// List projects at ssh server
  #[clap(visible_alias = "ls")]
  List {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
  },

  /// Remove project from server
  Remove {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
  },
}

#[derive(Args)]
struct ProjectOption {
  /// Unique project name
  #[clap(long, short)]
  project: Option<String>,
}

impl ops::Deref for ProjectOption {
  type Target = Option<String>;

  fn deref(&self) -> &Self::Target {
    &self.project
  }
}

#[derive(Args)]
struct SshAddressOption {
  /// Formats: login@ip, alias
  #[clap(long = "ssh")]
  ssh_address: Option<String>,
}

impl ops::Deref for SshAddressOption {
  type Target = Option<String>;

  fn deref(&self) -> &Self::Target {
    &self.ssh_address
  }
}

#[derive(Debug, Deserialize, Serialize)]
struct HmdYml {
  ssh_address: String,
  project: String,
  #[serde(default)]
  artifacts: Vec<String>,
  #[serde(flatten)]
  stages: IndexMap<String, String>,
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

#[derive(Debug, Deserialize, Serialize)]
struct HmdConfigYml {
  ssh_address: String,
}

impl HmdConfigYml {
  fn new(ssh_address: String) -> Self {
    Self { ssh_address }
  }
}

struct Env {
  project: String,
  ssh_address: String,
  project_dir: String,
  git_dir: String,
  work_tree: String,
}

impl Env {
  const OUT_LOG: &'static str = "out.log";
  const PIPELINE_PID: &'static str = "pipeline.pid";
  const PIPELINE_SH: &'static str = "pipeline.sh";
  const STATUS_LOG: &'static str = "status.log";

  fn new(project: &str, ssh_address: &str) -> Self {
    let project_dir = format!("{HMD_ROOT}/{project}");
    let git_dir = format!("{project_dir}/git");
    let work_tree = format!("{project_dir}/work-tree");
    Self {
      ssh_address: ssh_address.into(),
      project: project.into(),
      project_dir,
      git_dir,
      work_tree,
    }
  }

  fn out_log(&self) -> String {
    format!("{}/{}", self.work_tree, Self::OUT_LOG)
  }

  fn status_log(&self) -> String {
    format!("{}/{}", self.work_tree, Self::STATUS_LOG)
  }

  fn pipeline_pid(&self) -> String {
    format!("{}/{}", self.work_tree, Self::PIPELINE_PID)
  }
}

fn main() -> ExitCode {
  match launch() {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      eprintln!("{err}\n\n{err:?}");
      ExitCode::FAILURE
    }
  }
}

fn launch() -> io::Result<()> {
  match Cli::parse().command {
    Command::Init {
      ssh_address: SshAddressOption { ssh_address },
      project: ProjectOption { project },
    } => {
      let project = project
        .or_else(|| Some(read_hmd_yml().ok()?.project))
        .or_else(|| current_dir().ok())
        .ok_or(other_err(PROJECT_NOT_PROVIDED))?;
      init(&Env::new(&project, &get_ssh_address(ssh_address)?))
    }
    Command::Deploy { dirty } => {
      let hmd_yml = read_hmd_yml()?;
      let env = &Env::new(&hmd_yml.project, &hmd_yml.ssh_address);
      deploy(env, &hmd_yml, dirty)
    }
    Command::Stop {
      ssh_address: SshAddressOption { ssh_address },
      project: ProjectOption { project },
    } => {
      let project = get_project(project)?;
      stop(&Env::new(&project, &get_ssh_address(ssh_address)?))
    }
    Command::Restart {
      ssh_address: SshAddressOption { ssh_address },
      project: ProjectOption { project },
    } => {
      let project = get_project(project)?;
      let env = Env::new(&project, &get_ssh_address(ssh_address)?);
      restart_pipeline(&env)
    }
    Command::Status {
      ssh_address: SshAddressOption { ssh_address },
      project: ProjectOption { project },
    } => {
      let project = get_project(project)?;
      status(&Env::new(&project, &get_ssh_address(ssh_address)?))
    }
    Command::Log {
      ssh_address: SshAddressOption { ssh_address },
      project: ProjectOption { project },
    } => {
      let project = get_project(project)?;
      log(&Env::new(&project, &get_ssh_address(ssh_address)?))
    }
    Command::List {
      ssh_address: SshAddressOption { ssh_address },
    } => list(&get_ssh_address(ssh_address)?),
    Command::Remove {
      ssh_address: SshAddressOption { ssh_address },
      project,
    } => {
      let project =
        project.as_ref().ok_or(other_err(PROJECT_NOT_PROVIDED))?;
      remove(&Env::new(project, &get_ssh_address(ssh_address)?))
    }
  }
}

/// Searches ssh address in `ssh_address`, [`HMD_YML`] and [`HMD_CONFIG_YML`]
///
/// # Errors
///
/// Returns an error if ssh address not provided
fn get_ssh_address(
  ssh_address: Option<String>,
) -> io::Result<String> {
  ssh_address
    .or_else(|| Some(read_hmd_yml().ok()?.ssh_address))
    .or_else(|| Some(read_hmd_config_yml().ok()?.ssh_address))
    .ok_or(other_err("SSH address not provided"))
}

/// Searches project in `project` and [`HMD_YML`]
///
/// # Errors
///
/// Returns an error if project not provided
fn get_project(project: Option<String>) -> io::Result<String> {
  project
    .or_else(|| Some(read_hmd_yml().ok()?.project))
    .ok_or(other_err(PROJECT_NOT_PROVIDED))
}

fn current_dir() -> io::Result<String> {
  let error =
    || other_err("Can't parse project name from current dir");
  let project = env::current_dir()?
    .file_name()
    .ok_or_else(error)?
    .to_str()
    .ok_or_else(error)?
    .to_string();
  Ok(project)
}

fn init(env: &Env) -> io::Result<()> {
  init_srv_repo(env)?;
  write_hmd_yml(&env.project, &env.ssh_address)?;
  if read_hmd_config_yml().is_err() {
    write_hmd_config_yml(env.ssh_address.clone())?;
    println!("Config created at {HMD_CONFIG_YML}");
  }
  Ok(())
}

fn init_srv_repo(env: &Env) -> io::Result<()> {
  let ssh_address = &env.ssh_address;
  let git_dir = &env.git_dir;
  let work_tree = &env.work_tree;
  let ssh = &mut ssh(ssh_address);
  ssh
    .arg(format!("mkdir -p {git_dir} {work_tree};"))
    .arg(format!("cd {git_dir};"))
    .arg("git init --bare;");
  exec_verbose(ssh)?;
  Ok(())
}

fn ssh(ssh_address: &str) -> Cmd {
  let mut ssh = Cmd::new("ssh");
  ssh.arg(ssh_address);
  ssh
}

fn exec_verbose(cmd: &mut Cmd) -> io::Result<()> {
  let program = cmd.get_program().to_string_lossy();
  let args = cmd
    .get_args()
    .map(|arg| arg.to_string_lossy())
    .collect::<Vec<_>>()
    .join(" ");
  println!("\n{program} {args}");
  match cmd.status()? {
    status if status.success() => Ok(()),
    status => {
      Err(other_err(format!("Process terminated with {status}")))
    }
  }
}

fn write_hmd_yml(project: &str, ssh_address: &str) -> io::Result<()> {
  let hmd_yml = HmdYml {
    project: project.to_owned(),
    ssh_address: ssh_address.to_owned(),
    ..read_hmd_yml().unwrap_or_default()
  };
  let hmd_yml = serde_yaml::to_string(&hmd_yml).map_err(other_err)?;
  fs::write(HMD_YML, hmd_yml)?;
  Ok(())
}

fn write_hmd_config_yml(ssh_address: String) -> io::Result<()> {
  let hmd_config_yml = HmdConfigYml::new(ssh_address);
  let yml =
    serde_yaml::to_string(&hmd_config_yml).map_err(|err| {
      other_err(format!("Can't serialize hmd config: {err}"))
    })?;
  let home = env::var("HOME").map_err(other_err)?;
  fs::create_dir_all(HMD_ROOT.replacen('~', &home, 1))?;
  fs::write(HMD_CONFIG_YML.replacen('~', &home, 1), yml)?;
  Ok(())
}

fn read_hmd_config_yml() -> io::Result<HmdConfigYml> {
  let home = env::var("HOME").map_err(other_err)?;
  let yml =
    fs::read_to_string(HMD_CONFIG_YML.replacen('~', &home, 1))?;
  let hmd_config_yml: HmdConfigYml = serde_yaml::from_str(&yml)
    .map_err(|err| {
      other_err(format!("Can't read hmd config: {err}"))
    })?;
  Ok(hmd_config_yml)
}

fn read_hmd_yml() -> io::Result<HmdYml> {
  let yml =
    fs::read_to_string(HMD_YML).map_err(hmd_yml_not_found_context)?;
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

fn hmd_yml_not_found_context(err: io::Error) -> io::Error {
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

fn deploy(
  env: &Env,
  hmd_yml: &HmdYml,
  dirty: bool,
) -> io::Result<()> {
  if dirty {
    git_push_dirty(env)?;
  } else {
    git_push(env)?;
  }
  generate_pipeline_sh(&hmd_yml.stages)?;
  let mut artifacts = hmd_yml.artifacts.clone();
  artifacts.push(Env::PIPELINE_SH.to_string());
  upload(env, &artifacts)?;
  fs::remove_file(Env::PIPELINE_SH)?;
  run_pipeline(env)?;
  Ok(())
}

fn git_push_dirty(env: &Env) -> io::Result<()> {
  git_commit_staged()?;
  git_commit_unstaged()?;
  let push_result = git_push(env);
  git_reset_unstaged()?;
  git_reset_staged()?;
  push_result?;
  Ok(())
}

fn git_commit_staged() -> io::Result<()> {
  run_verbose("git commit -m staged --allow-empty")?;
  Ok(())
}

fn git_commit_unstaged() -> io::Result<()> {
  run_verbose("git add .")?;
  run_verbose("git commit -m unstaged --allow-empty")?;
  Ok(())
}

fn git_reset_unstaged() -> io::Result<()> {
  run_verbose("git reset HEAD~1")?;
  Ok(())
}

fn git_reset_staged() -> io::Result<()> {
  run_verbose("git reset HEAD~1 --soft")?;
  Ok(())
}

fn git_push(env: &Env) -> io::Result<()> {
  let ssh_address = &env.ssh_address;
  let git_dir = &env.git_dir;
  run_verbose(&format!(
    "git push --force {ssh_address}:{git_dir} HEAD"
  ))?;
  Ok(())
}

fn run_verbose(args: &str) -> io::Result<()> {
  let args = &mut args.split_whitespace();
  let mut cmd = Cmd::new(args.next().unwrap());
  cmd.args(args);
  exec_verbose(&mut cmd)
}

fn generate_pipeline_sh(
  stages: &IndexMap<String, String>,
) -> io::Result<()> {
  let pipeline = stage_commands(stages).join("\n\n");
  let stages = stages.keys().cloned().collect::<Vec<_>>().join(" ");
  let script =
    format!("{SCRIPT}\n\nstages=({stages});\n\n{pipeline}");
  fs::write(Env::PIPELINE_SH, script)?;
  Ok(())
}

fn upload(env: &Env, artifacts: &[String]) -> io::Result<()> {
  if artifacts.is_empty() {
    return Ok(());
  }
  let ssh_address = &env.ssh_address;
  let work_tree = &env.work_tree;
  let scp = &mut Cmd::new("scp");
  scp
    .args(artifacts)
    .arg(format!("{ssh_address}:{work_tree}"));
  exec_verbose(scp)?;
  Ok(())
}

fn run_pipeline(env: &Env) -> io::Result<()> {
  let out_log = Env::OUT_LOG;
  let pipeline_sh = Env::PIPELINE_SH;
  let pipeline_pid = Env::PIPELINE_PID;
  let work_tree = &env.work_tree;
  // FIXME: Why git doesn't recognize ~ path?
  let git_dir = env.git_dir.replacen('~', "$HOME", 1);
  let branch = git_branch()?;
  let ssh = &mut ssh(&env.ssh_address);
  ssh
    .arg("source .profile;")
    .arg(format!("cd {work_tree};"))
    .arg(kill_and_wait_cmd(pipeline_pid))
    .arg(format!(
      "git --git-dir={git_dir} --work-tree=. checkout --force {branch};"
    ))
    .arg(format!(
      "nohup bash {pipeline_sh} > {out_log} 2>&1 & echo $! > {pipeline_pid};"
    ));
  exec_verbose(ssh)?;
  Ok(())
}

fn restart_pipeline(env: &Env) -> io::Result<()> {
  let out_log = Env::OUT_LOG;
  let pipeline_sh = Env::PIPELINE_SH;
  let pipeline_pid = Env::PIPELINE_PID;
  let work_tree = &env.work_tree;
  let ssh = &mut ssh(&env.ssh_address);
  ssh
    .arg("source .profile;")
    .arg(format!("cd {work_tree};"))
    .arg(kill_and_wait_cmd(pipeline_pid))
    .arg(format!(
      "nohup bash {pipeline_sh} > {out_log} 2>&1 & echo $! > {pipeline_pid};"
    ));
  exec_verbose(ssh)?;
  Ok(())
}

fn git_branch() -> Result<String, io::Error> {
  let output = Cmd::new("git")
    .args(["branch", "--show-current"])
    .output()?;
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn stage_commands(stages: &IndexMap<String, String>) -> Vec<String> {
  stages
    .iter()
    .enumerate()
    .map(|(i, (stage, cmd))| {
      format!(
        r#"
          echo -e "\n🟩 [`date +%FT%T`] > Start {stage}\n{cmd}\n";
          run {i} && {cmd} && complete {i} || {{
            echo -e "\n❌ [`date +%FT%T`] > Failed {stage}\n";
            panic {i};
            exit 1;
          }};
          echo -e "\n🟩 [`date +%FT%T`] > End {stage}\n";
        "#
      )
    })
    .collect()
}

fn stop(env: &Env) -> io::Result<()> {
  let ssh = &mut ssh(&env.ssh_address);
  ssh.arg(kill_and_wait_cmd(&env.pipeline_pid()));
  exec_verbose(ssh)?;
  Ok(())
}

fn kill_and_wait_cmd(pipeline_pid: &str) -> String {
  format!(
    "while pkill -SIGINT -P `cat {pipeline_pid}` 2>/dev/null; do sleep 1; done;"
  )
}

fn status(env: &Env) -> io::Result<()> {
  let ssh = &mut ssh(&env.ssh_address);
  ssh.arg(format!("tail -f {}", env.status_log()));
  exec_verbose(ssh)?;
  Ok(())
}

fn log(env: &Env) -> io::Result<()> {
  let ssh = &mut ssh(&env.ssh_address);
  ssh.arg(format!("tail -n 50 -f {}", env.out_log()));
  exec_verbose(ssh)?;
  Ok(())
}

fn list(ssh_address: &str) -> io::Result<()> {
  let ssh = &mut ssh(ssh_address);
  ssh.arg(format!("ls {HMD_ROOT}"));
  exec_verbose(ssh)?;
  Ok(())
}

fn remove(env: &Env) -> io::Result<()> {
  let project_dir = &env.project_dir;
  let ssh = &mut ssh(&env.ssh_address);
  ssh
    .arg(kill_and_wait_cmd(&env.pipeline_pid()))
    .arg(format!("rm -rf {project_dir}"));
  exec_verbose(ssh)?;
  Ok(())
}

fn other_err<E>(err: E) -> io::Error
where
  E: Into<Box<dyn Error + Send + Sync>>,
{
  io::Error::other(err)
}
