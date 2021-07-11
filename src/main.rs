use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;

mod detoxer;

use detoxer::*;

pub const NUM_THREADS: usize = 3;

#[derive(Debug, StructOpt)]
#[structopt(name = "downloader")]
struct Cli {
    /// Runs without naming files.
    #[structopt(short, long)]
    dry_run: bool,

    /// Logs file names that are changing.
    #[structopt(long)]
    verbose: bool,

    /// Renames hidden files.
    #[structopt(long)]
    hidden: bool,

    /// Moves files that overlap with existing files to another location.
    #[structopt(long = "move", parse(from_os_str))]
    move_to: Option<PathBuf>,

    /// Files/directories to rename.
    #[structopt(parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let Cli {
        dry_run,
        verbose,
        hidden: traverse_hidden,
        move_to,
        inputs,
    } = Cli::from_args();

    let options = Arc::new(Options {
        dry_run,
        verbose,
        hidden: traverse_hidden,
        move_to,
    });
    let inputs = Arc::new(Mutex::new(inputs));

    let handles = (0..NUM_THREADS)
        .into_iter()
        .map(|_| {
            let options = Arc::clone(&options);
            let inputs = Arc::clone(&inputs);

            thread::spawn(move || -> Result<()> { detox(inputs, options) })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap()?;
    }

    Ok(())
}
