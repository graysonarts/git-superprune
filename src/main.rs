use std::{env, sync::atomic::AtomicU8};

use branch::BranchList;
use clap::Parser;
use color_eyre::Result;
use const_format::formatcp;
use git2::{Cred, Repository};
use git_version::git_version;
use tracing::{debug, info, warn};

mod branch;

const GIT_VERSION: &str = git_version!(
    prefix = "git:",
    cargo_prefix = "cargo:",
    fallback = "unknown"
);
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION: &str = formatcp!("v{CRATE_VERSION} ({GIT_VERSION})");

#[derive(Debug, Parser)]
#[clap(
    name = "git-superprune",
    version = VERSION,
    about = "Prune local branches that no longer exist on the remote",
)]
struct Args {
    #[clap(short, long, default_value = "false")]
    /// Display verbose output
    verbose: bool,

    /// run `git remote prune <upstream>` before scanning
    #[clap(short, long, required = false, env = "SUPERPRUNE_UPSTREAM_REMOTE")]
    upstream: Option<String>,

    /// SSH key in `~/.ssh/`` to use for authentication with remote, defaults to `id_rsa`
    #[clap(short, long, required = false, env = "SUPERPRUNE_SSH_KEY")]
    ssh_key: Option<String>,

    #[clap(short('x'), long, default_value = "false")]
    /// Execute the branch deletion
    execute: bool,

    /// Root directory of the git repository
    #[clap(required = false)]
    root: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let format = tracing_subscriber::fmt::format()
        .without_time()
        .with_target(false);

    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .event_format(format)
        .init();

    let repository = Repository::discover(args.root.as_deref().unwrap_or("."))?;

    if let Some(upstream) = &args.upstream {
        debug!("Finding upstream {upstream}");
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            let key_name = args.ssh_key.as_deref().unwrap_or("id_rsa");
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/{key_name}", env::var("HOME").unwrap())),
                None,
            )
        });
        let mut remote = repository.find_remote(upstream)?;
        remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;
        debug!(
            "Pruning remote {upstream} {}",
            remote.url().unwrap_or_default()
        );
        remote.prune(None)?;
    }

    debug!("Scanning Repository");

    let dry_run = !args.execute;
    let deleted = AtomicU8::new(0);

    let branches = repository
        .get_branches()
        .into_iter()
        .inspect(|b| {
            debug!("Checking branch: {}", b.name);
        })
        .filter(|b| b.gone);
    for branch in branches {
        if dry_run {
            info!("Would delete branch: {}", branch.name);
            deleted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            match delete_branch(&repository, &branch.name) {
                Ok(()) => {
                    info!("Deleted branch: {}", branch.name);
                    deleted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                // Don't let one undeletable branch (e.g. checked out in a linked
                // worktree) abort the whole run — skip it and keep pruning.
                Err(e) => warn!("Skipping branch {}: {}", branch.name, e),
            }
        }
    }

    if deleted.load(std::sync::atomic::Ordering::Relaxed) == 0 {
        info!("No branches to prune");
        return Ok(());
    }

    if dry_run {
        info!(
            "Would have deleted {} branches, rerun with -x to execute",
            deleted.load(std::sync::atomic::Ordering::Relaxed)
        );
    } else {
        info!(
            "Pruned {} branches",
            deleted.load(std::sync::atomic::Ordering::Relaxed)
        );
    }

    Ok(())
}

/// Delete a local branch, tolerating multivar config entries.
///
/// `git2::Branch::delete` (libgit2's `git_branch_delete`) removes the
/// `[branch "<name>"]` config section before deleting the ref. That removal
/// fails with `entry is not unique due to being a multivar` when tools such as
/// the GitHub CLI (`github-pr-owner-number`) or VS Code (`vscode-merge-base`)
/// have written duplicate keys into the section. When that happens we clean up
/// the config ourselves in a multivar-safe way and delete the ref directly.
fn delete_branch(repository: &Repository, name: &str) -> Result<()> {
    let mut branch = repository.find_branch(name, git2::BranchType::Local)?;
    match branch.delete() {
        Ok(()) => Ok(()),
        Err(e) if e.class() == git2::ErrorClass::Config => {
            debug!("Branch delete hit multivar config, cleaning up manually: {e}");
            remove_branch_config(repository, name)?;
            branch.into_reference().delete()?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Remove every config entry under the `[branch "<name>"]` section, including
/// multivar (duplicate) keys that `git_config_delete_entry` refuses to remove.
fn remove_branch_config(repository: &Repository, name: &str) -> Result<()> {
    let mut config = repository.config()?;
    let pattern = format!("^branch\\.{}\\.", regex::escape(name));

    let mut keys = std::collections::HashSet::new();
    config.entries(Some(&pattern))?.for_each(|entry| {
        if let Ok(key) = entry.name() {
            keys.insert(key.to_string());
        }
    })?;

    for key in keys {
        // `remove_multivar` with a match-all value regex clears the key whether
        // it holds one value or many, unlike `remove`, which errors on multivars.
        config.remove_multivar(&key, ".*")?;
    }
    Ok(())
}
