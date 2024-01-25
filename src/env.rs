use crate::HMD_ROOT;

pub(crate) struct Env {
  pub(crate) project: String,
  pub(crate) ssh_address: String,
  pub(crate) project_dir: String,
  pub(crate) git_dir: String,
  pub(crate) work_tree: String,
}

impl Env {
  pub(crate) const OUT_LOG: &'static str = "out.log";
  pub(crate) const PIPELINE_PID: &'static str = "pipeline.pid";
  pub(crate) const PIPELINE_SH: &'static str = "pipeline.sh";
  pub(crate) const STATUS_LOG: &'static str = "status.log";

  pub(crate) fn new(project: &str, ssh_address: &str) -> Self {
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

  pub(crate) fn out_log(&self) -> String {
    format!("{}/{}", self.work_tree, Self::OUT_LOG)
  }

  pub(crate) fn status_log(&self) -> String {
    format!("{}/{}", self.work_tree, Self::STATUS_LOG)
  }

  pub(crate) fn pipeline_pid(&self) -> String {
    format!("{}/{}", self.work_tree, Self::PIPELINE_PID)
  }
}
