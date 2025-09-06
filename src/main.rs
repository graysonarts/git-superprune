use std::{env, sync::atomic::AtomicU8};

use branch::BranchList;
use clap::Parser;
use color_eyre::Result;
use const_format::formatcp;
use git2::{Cred, Repository};
use git_version::git_version;
use tracing::{debug, info};

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
            b
        })
        .filter(|b| b.gone);
    for branch in branches {
        if dry_run {
            info!("Would delete branch: {}", branch.name);
            deleted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            let mut git_branch = repository.find_branch(&branch.name, git2::BranchType::Local)?;
            git_branch.delete()?;
            info!("Deleted branch: {}", branch.name);
            deleted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
