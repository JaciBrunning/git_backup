Git Backup
======

Quick rust program to backup repositories from Github and Gitlab. Inspired by [gickup](https://github.com/cooperspencer/gickup).

# What it does
This project will poll github and gitlab services for a list of repositories available to your user (the bearer of your access token) and clone them into the output directory. If the target already exists, the repository will be updated from the remote.

The file structure is `{target}/{source}/{owner}/{name}`, where `target` is loaded from the configuration below and `source` is either 'github', or 'gitlab' (or a custom defined 'name' in the case of gitlab).

Cloned repositories are bare repos. You can clone from these repositories by adding the file path as a remote, or push them directly to a remote in order to restore the backup. The repository is bare as to avoid unecessary extra data usage by checking out the git tree, but rather storing the packed objects.

Note the local tree will be perfectly in-sync with the remote. If the remote is the subject of a force-push, the mirrored repos created with this tool will be also. Thus, it is possible for data to be 'lost' upon replication if the git history reverts. The intention is to use this tool periodically to update a local set of repositories that are also periodically snapshotted using a filesystem such as ZFS.

# Usage
## Standalone
Build the project with `cargo`
```sh
cargo build --release
./target/release/git_backup --help
```

## Docker
```sh
docker run -it --rm -v "/path/to/config.yml:/work/config.yml:ro" -v "/home/user/.ssh/id_rsa:/root/.ssh/id_rsa:ro" jaci/git_backup:latest
```

Note: `ssh` volume is only required if you plan to clone private repositories using SSH. If all repositories are public, there is no need for SSH authentication.

## Configuration
The configuration file `config.yml` follows the layout below. You can use more than one github / gitlab source if desired.

```yml
---
sources:
  - github:
    user: JaciBrunning
    token: your_personal_access_token_here
    clone: SSH  # SSH or HTTPS
    forks: true # Disable to avoid cloning forks
    exclude: # Optional
      - my_repo
    exclude_owners: # Optional
      - my_org
  
  - gitlab:
      name: my_gitlab # Optional. Defaults to gitlab
      url: https://gitlab.com/  # Optional. Defaults to https://gitlab.com/. Change if you're using a self-hosted gitlab instance
      token: your_personal_access_token_here
      clone: SSH # SSH or HTTPS
      forks: true # Disable to avoid cloning forks
      exclude:  # Optional
        - my_repo
      exclude_owners: # Optional
        - my_org

target: "./my_backup_dir/"
```