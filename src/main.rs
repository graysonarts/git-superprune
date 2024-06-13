use std::sync::atomic::AtomicU8;

use branch::BranchList;
use clap::Parser;
use color_eyre::Result;
use const_format::formatcp;
use git2::Repository;
use git_version::git_version;
use tracing::{debug, info};
use tracing_subscriber::field::debug;

mod branch;

const GIT_VERSION: &str = git_version!();
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

    #[clap(short('x'), long, default_value = "false")]
    /// Execute the branch deletion
    execute: bool,
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

    debug!("Scanning Repository");

    let dry_run = !args.execute;
    let deleted = AtomicU8::new(0);

    let repository = Repository::discover(".")?;
    let branches = repository
        .get_branches()
        .into_iter()
        .map(|b| {
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
