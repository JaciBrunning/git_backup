use log::{error, info};
use reqwest::RequestBuilder;
use serde_json::Value;

use crate::git::{CloneType, RepoProvider, GitRepo};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GitlabSource {
  pub name: Option<String>,
  pub url: Option<String>,
  // pub user: String,
  pub token: String,
  pub clone: CloneType,
  pub forks: bool,

  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub exclude_owners: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub exclude: Vec<String>
}

#[derive(serde::Deserialize, Debug)]
pub struct GitlabForkedFrom {
  id: usize
}

#[derive(serde::Deserialize, Debug)]
pub struct GitlabNamespace {
  path: String
}

#[derive(serde::Deserialize, Debug)]
pub struct GitlabRepo {
  path: String,
  path_with_namespace: String,
  ssh_url_to_repo: String,
  http_url_to_repo: String,
  forked_from_project: Option<GitlabForkedFrom>,
  namespace: GitlabNamespace
}

impl GitlabSource {
  async fn query<F>(&self, fragment: &[&str], build: F) -> anyhow::Result<Value>
    where F: Fn(RequestBuilder) -> RequestBuilder {
    let req_url = format!("{}/api/v4/{}", self.url.as_deref().unwrap_or("https://gitlab.com"), fragment.join("/"));
    let client = reqwest::Client::new();
    let mut builder = client.get(&req_url)
      .header("User-Agent", "git_backup")
      .header("PRIVATE-TOKEN", &self.token);
      // .basic_auth(self.user.clone(), Some(self.token.clone()));
    builder = build(builder);
    let response = builder.send().await?;
    match response.status().is_success() {
      true => Ok(response.json::<Value>().await?),
      false => anyhow::bail!("Uh oh! Status code: {}", response.status())
    }
  }

  async fn paginate<F>(&self, fragment: &[&str], build: F) -> anyhow::Result<Vec<Value>> 
    where F: Fn(RequestBuilder) -> RequestBuilder {
    let mut page = 1;
    let mut list = vec![];
    let mut more = true;

    while more {
      let response: Vec<Value> = serde_json::from_value(
        self.query(fragment, |bdr| 
          build(bdr.query(&[("per_page", "100"), ("page", &page.to_string())]))
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

    for repo in self.paginate(&vec!["projects"], |bdr| bdr.query(&[("membership", "true")])).await? {
      let gl_repo_typed: GitlabRepo = serde_json::from_value(repo)?;
      if (!self.forks && gl_repo_typed.forked_from_project.is_some()) || self.exclude_owners.contains(&gl_repo_typed.namespace.path) || self.exclude.contains(&gl_repo_typed.path) {
        excluded.push(gl_repo_typed);
      } else {
        list.push(GitRepo {
          source: self.name.clone().unwrap_or("gitlab".to_owned()),
          owner: gl_repo_typed.namespace.path,
          name: gl_repo_typed.path,
          url: match self.clone {
            CloneType::SSH => gl_repo_typed.ssh_url_to_repo.clone(),
            CloneType::HTTPS => gl_repo_typed.http_url_to_repo.clone(),
          }
        });
      }
    }

    if excluded.len() > 20 {
      info!("Excluded {} repositories", excluded.len());
    } else {
      info!("Excluded {} repositories: {:?}", excluded.len(), excluded.iter().map(|gr| gr.path_with_namespace.as_str()).collect::<Vec<&str>>());
    }

    Ok(list)
  }
}

#[async_trait::async_trait]
impl RepoProvider for GitlabSource {
  async fn repos(&self) -> Vec<crate::git::GitRepo> {
    info!("GitLab: Querying repos...");
    match self.user_repos().await {
      Ok(repos) => repos,
      Err(err) => {
        error!("GitLab error: {}", err);
        vec![]
      },
    }
  }
}