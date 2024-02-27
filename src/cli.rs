use clap::{Args, Parser, Subcommand};

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
pub(crate) struct Cli {
  #[clap(subcommand)]
  pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
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

  /// Open working dir at ssh server
  #[clap(visible_alias = "o")]
  Open {
    #[clap(flatten)]
    ssh_address: SshAddressOption,
    #[clap(flatten)]
    project: ProjectOption,
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
pub(crate) struct ProjectOption {
  /// Unique project name
  #[clap(long, short)]
  pub(crate) project: Option<String>,
}

#[derive(Args)]
pub(crate) struct SshAddressOption {
  /// Formats: login@ip, alias
  #[clap(long = "ssh")]
  pub(crate) ssh_address: Option<String>,
}
