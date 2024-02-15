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
//! - [x] Split by modules
//! - [ ] Notifications via Telegram Bot
//! - [ ] Scope option
//! - [ ] Verbose option
//! - [ ] Pretty logs
//! - [ ] Publish crate
//! - [ ] Completions
//! - [ ] Man
//! - [ ] Update README
//! - [ ] Status option for deploy
//! - [ ] Log option for deploy
//! - [ ] Config path option
//!

mod cli;
mod env;
mod hmd_config_yml;
mod hmd_yml;

use std::{
  error::Error,
  fs, io,
  process::{Command as Cmd, ExitCode},
};

use clap::Parser;
use indexmap::IndexMap;

use crate::{
  cli::{Cli, Command, ProjectOption, SshAddressOption},
  env::Env,
  hmd_config_yml::HMD_CONFIG_YML,
  hmd_yml::HmdYml,
};

const HMD_ROOT: &str = "~/.hmd";
const SCRIPT: &str = include_str!("../script.sh");
const PROJECT_NOT_PROVIDED: &str = "Project not provided";

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
        .or_else(|| Some(hmd_yml::read().ok()?.project))
        .or_else(|| current_dir().ok())
        .ok_or(other_err(PROJECT_NOT_PROVIDED))?;
      init(&Env::new(&project, &get_ssh_address(ssh_address)?))
    }
    Command::Deploy { dirty } => {
      let hmd_yml = hmd_yml::read()?;
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
      project: ProjectOption { project },
    } => {
      let project =
        project.as_ref().ok_or(other_err(PROJECT_NOT_PROVIDED))?;
      remove(&Env::new(project, &get_ssh_address(ssh_address)?))
    }
  }
}

/// Searches ssh address in `ssh_address`, `hmd.yml` and `~/.hmd/config.yml`
///
/// # Errors
///
/// Returns an error if ssh address not provided
fn get_ssh_address(
  ssh_address: Option<String>,
) -> io::Result<String> {
  ssh_address
    .or_else(|| Some(hmd_yml::read().ok()?.ssh_address))
    .or_else(|| Some(hmd_config_yml::read().ok()?.ssh_address))
    .ok_or(other_err("SSH address not provided"))
}

/// Searches project in `project` and `hmd.yml`
///
/// # Errors
///
/// Returns an error if project not provided
fn get_project(project: Option<String>) -> io::Result<String> {
  project
    .or_else(|| Some(hmd_yml::read().ok()?.project))
    .ok_or(other_err(PROJECT_NOT_PROVIDED))
}

fn current_dir() -> io::Result<String> {
  let error =
    || other_err("Can't parse project name from current dir");
  let project = std::env::current_dir()?
    .file_name()
    .ok_or_else(error)?
    .to_str()
    .ok_or_else(error)?
    .to_string();
  Ok(project)
}

fn init(env: &Env) -> io::Result<()> {
  init_srv_repo(env)?;
  hmd_yml::write(&env.project, &env.ssh_address)?;
  if hmd_config_yml::read().is_err() {
    hmd_config_yml::write(env.ssh_address.clone())?;
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

fn git_branch() -> io::Result<String> {
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
  let status_log = env.status_log();
  let pid = env.pipeline_pid();
  ssh.arg(format!("tail -f {status_log} --pid `cat {pid}`"));
  exec_verbose(ssh)?;
  Ok(())
}

fn log(env: &Env) -> io::Result<()> {
  let ssh = &mut ssh(&env.ssh_address);
  let log = env.out_log();
  let pid = env.pipeline_pid();
  ssh.arg(format!("tail -n 50 -f {log} --pid `cat {pid}`"));
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
