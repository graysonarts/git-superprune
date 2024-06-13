use clap::Parser;
use git_version::git_version;

#[derive(Debug, Parser)]
struct Args {}

const GIT_VERSION: &str = git_version!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("{:?}", args);
    Ok(())
}
