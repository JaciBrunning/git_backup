use std::path::Path;

use git2::Repository;
use log::{info, error};

use crate::{config::{GitBackupSettings, Source}, git::RepoProvider};

pub mod git;
pub mod config;
pub mod github;
pub mod gitlab;

async fn process_repo(repo: &git::GitRepo, root: &Path) {
  let target_dir = root.join(&repo.source).join(&repo.owner).join(&repo.name);
  let target_dir = target_dir.as_path();

  match Repository::open(target_dir) {
    Ok(mut r) => {
      info!("Updating Repo {}", &repo.url);
      match repo.update(&mut r) {
        Ok(updates) => match updates {
          Some(n_objects) => info!("Updated. Received {} total objects.", n_objects),
          None => info!("Already up-to-date!"),
        },
        Err(err) => {
          error!("Repo: {:?} => Update error: {}", &repo.url, err);
        },
      }
    },
    Err(_) => {
      info!("Mirroring Repo {}", &repo.url);
      match repo.mirror(target_dir) {
        Ok(()) => info!("Mirrored!"),
        Err(err) => {
          error!("Repo: {:?} => Mirror error: {}", &repo.url, err)
        },
      }
    },
  }
}

async fn process_repos(repos: Vec<git::GitRepo>, root: &Path) {
  for repo in repos {
    process_repo(&repo, root).await;
  }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  pretty_env_logger::init();

  let f = std::fs::File::open("config.yml")?;
  let cfg: GitBackupSettings = serde_yaml::from_reader(f)?;

  let root = Path::new(&cfg.target);

  for source in &cfg.sources {
    info!("Processing Source: {:?}", source);
    match source {
      Source::Github(gh) => process_repos(gh.repos().await, root).await,
      Source::Gitlab(gl) => process_repos(gl.repos().await, root).await
    }
  }

  Ok(())
}
