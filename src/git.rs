use std::{path::Path, env};

use git2::{build::RepoBuilder, RemoteCallbacks, FetchOptions};
use log::debug;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum CloneType {
  SSH, HTTPS
}

#[derive(Debug, Clone)]
pub struct GitRepo {
  pub source: String,
  pub owner: String,
  pub name: String,
  pub url: String
}

#[async_trait::async_trait]
pub trait RepoProvider {
  async fn repos(&self) -> Vec<GitRepo>;
}

pub fn ssh_privkey_auth(_usr: &str, usr_url: Option<&str>, _credtype: git2::CredentialType) -> Result<git2::Cred, git2::Error> {
  git2::Cred::ssh_key(usr_url.unwrap(), None, std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())), None)
}

impl GitRepo {
  pub fn mirror(&self, path: &Path) -> anyhow::Result<()> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(ssh_privkey_auth);

    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks);
    opts.download_tags(git2::AutotagOption::All);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(opts);
    builder.bare(true);
    builder.remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

    builder.clone(&self.url, path)?;
    Ok(())
  }

  pub fn update(&self, repo: &mut git2::Repository) -> anyhow::Result<Option<usize>> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(ssh_privkey_auth);
    
    let mut remote = repo.find_remote("origin")?;
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks);
    opts.download_tags(git2::AutotagOption::All);

    remote.download(&[] as &[&str], Some(&mut opts))?;

    let stats = remote.stats();
    let total_objects = stats.total_objects();
    debug!("Received {}/{} objects in {} bytes", stats.indexed_objects(), stats.total_objects(), stats.received_bytes());

    remote.disconnect()?;
    remote.update_tips(None, true, git2::AutotagOption::All, None)?;

    if total_objects > 0 {
      Ok(Some(total_objects))
    } else {
      Ok(None)
    }
  }
}