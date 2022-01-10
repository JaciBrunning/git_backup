use std::{path::{Path, PathBuf}, process::exit};

use clap::Parser;
use futures::future::join_all;
use git2::Repository;
use log::{info, error};
use tokio::task;

use crate::{config::{GitBackupSettings, Source}, git::RepoProvider};

pub mod git;
pub mod config;
pub mod github;
pub mod gitlab;

async fn process_repo(repo: &git::GitRepo, target: String) {
  let target_dir = Path::new(&target).join(&repo.source).join(&repo.owner).join(&repo.name);
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

async fn process_repos(repos: Vec<git::GitRepo>, root: String) {
  let join_handles = repos.into_iter().map(|repo| {
    let target = root.clone();
    task::spawn(async move { process_repo(&repo, target).await })
  });

  join_all(join_handles).await;
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
  #[clap(short, long, default_value = "config.yml")]
  config: PathBuf
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  if let Err(_) = std::env::var("RUST_LOG") {
    std::env::set_var("RUST_LOG", "info");
  }

  pretty_env_logger::init();
  let args = Args::parse();

  match std::fs::File::open(&args.config) {
    Ok(f) => {
      let cfg: GitBackupSettings = serde_yaml::from_reader(f)?;

      let join_handles = cfg.sources.into_iter().map(|source| {
        let target = cfg.target.clone();
        task::spawn(async move {
          info!("Processing Source: {:?}", source);
          match source {
            Source::Github(gh) => process_repos( gh.repos().await, target).await,
            Source::Gitlab(gl) => process_repos(gl.repos().await, target).await,
          };
        })
      });

      join_all(join_handles).await;
    },
    Err(err) => {
      error!("Could not open config file {}: {}", args.config.display(), err);
      exit(1)
    },
  }

  Ok(())
}
