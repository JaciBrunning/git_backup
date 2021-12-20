use crate::{github::GithubSource, gitlab::GitlabSource};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Source {
  Github(GithubSource),
  Gitlab(GitlabSource)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GitBackupSettings {
  pub sources: Vec<Source>,
  pub target: String
}