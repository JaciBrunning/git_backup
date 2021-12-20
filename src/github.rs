use log::{error, info};
use reqwest::RequestBuilder;
// use github_rs::client::{Github, Executor};
use serde_json::Value;

use crate::git::{CloneType, RepoProvider, GitRepo};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GithubSource {
  pub user: String,
  pub token: String,
  pub clone: CloneType,
  pub forks: bool,

  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub exclude_owners: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub exclude: Vec<String>
}

#[derive(serde::Deserialize, Debug)]
pub struct GithubRepoOwner {
  login: String
}

#[derive(serde::Deserialize, Debug)]
pub struct GithubRepo {
  owner: GithubRepoOwner,
  name: String,
  full_name: String,
  ssh_url: String,
  clone_url: String,
  fork: bool
}

impl GithubSource {
  async fn query<F>(&self, fragment: &[&str], build: F) -> anyhow::Result<Value>
    where F: Fn(RequestBuilder) -> RequestBuilder {
    let req_url = format!("https://api.github.com/{}", fragment.join("/"));
    let client = reqwest::Client::new();
    let mut builder = client.get(&req_url)
      .header("User-Agent", "git_backup")
      .basic_auth(self.user.clone(), Some(self.token.clone()));
    builder = build(builder);
    let response = builder.send().await?;
    match response.status().is_success() {
      true => Ok(response.json::<Value>().await?),
      false => anyhow::bail!("Uh oh! Status code: {}", response.status())
    }
  }

  async fn paginate(&self, fragment: &[&str]) -> anyhow::Result<Vec<Value>> {
    let mut page = 1;
    let mut list = vec![];
    let mut more = true;

    while more {
      let response: Vec<Value> = serde_json::from_value(
        self.query(fragment, |bdr| 
          bdr.query(&[("per_page", "100"), ("page", &page.to_string())])
        ).await?
      )?;
      more = response.len() == 100;
      page += 1;
      list.extend(response.iter().cloned());
    }

    Ok(list)
  }

  async fn user_repos(&self) -> anyhow::Result<Vec<GitRepo>> {
    let mut list = vec![];
    let mut excluded = vec![];

    for gh_repo in self.paginate(&vec!["user", "repos"]).await? {
      let gh_repo_typed: GithubRepo = serde_json::from_value(gh_repo)?;
      if (!self.forks && gh_repo_typed.fork) || self.exclude_owners.contains(&gh_repo_typed.owner.login) || self.exclude.contains(&gh_repo_typed.name) {
        excluded.push(gh_repo_typed);
      } else {
        list.push(GitRepo {
          source: "github".to_owned(), 
          owner: gh_repo_typed.owner.login, 
          name: gh_repo_typed.name,
          url: match self.clone {
            CloneType::SSH => gh_repo_typed.ssh_url.clone(),
            CloneType::HTTPS => gh_repo_typed.clone_url.clone(),
          }
        });
      }
    }

    if excluded.len() > 20 {
      info!("Excluded {} repositories", excluded.len());
    } else {
      info!("Excluded {} repositories: {:?}", excluded.len(), excluded.iter().map(|ghr| ghr.full_name.as_str()).collect::<Vec<&str>>());
    }

    Ok(list)
  }
}

#[async_trait::async_trait]
impl RepoProvider for GithubSource {
  async fn repos(&self) -> Vec<crate::git::GitRepo> {
    info!("GitHub: Querying repos...");
    match self.user_repos().await {
      Ok(repos) => repos,
      Err(err) => {
        error!("GitHub error: {}", err);
        vec![]
      },
    }
  }
}