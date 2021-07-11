use anyhow::{anyhow, Result};
use path_clean::PathClean;
use regex::Regex;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct Detoxer {
    replacements: Vec<(Regex, String)>,
}

impl Detoxer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            replacements: vec![
                // Remove non unicode characters
                // https://stackoverflow.com/questions/150033/regular-expression-to-match-non-ascii-characters
                // TODO: test
                (Regex::new("[^\x00-\x7F]+")?, "".to_string()),
                // Replace spaces, _, -
                (Regex::new(r"[-_\s]+")?, "-".to_string()),
                // Trim, remove commas
                (Regex::new(r"(,+)|(^-+)|(-+$)")?, "".to_string()),
                // Trim "-"
                (Regex::new(r"(^-+)|(-+$)")?, "".to_string()),
            ],
        })
    }

    pub fn detox_file_name(&self, file_name: &str) -> String {
        self.replacements
            .iter()
            .fold(file_name.to_string(), |prev, (regex, replacement)| {
                regex.replace_all(&prev, replacement).to_string()
            })
    }
}

fn hidden(path: &PathBuf) -> bool {
    path.file_name()
        .map(|entry| entry.to_str().unwrap().starts_with("."))
        .unwrap_or(false)
}

pub struct Options {
    pub dry_run: bool,
    pub verbose: bool,
    pub hidden: bool,
    pub move_to: Option<PathBuf>,
}

fn absolute_path(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    }
    .clean();

    Ok(absolute_path)
}

fn move_file_to(from: &PathBuf, to: &PathBuf) -> Result<()> {
    let from = absolute_path(&from)?;
    let mut to = absolute_path(&to)?;

    to.push(from.file_name().unwrap());

    fs::create_dir_all(to.parent().ok_or(anyhow!("Invalid move path."))?)?;
    fs::rename(&from, &to)?;

    println!("{:?}\n{:?}", from, to);

    Ok(())
}

/// Detoxes a **file** (does not handle directories).
fn detox_file(input: PathBuf, detoxer: &Detoxer, options: &Arc<Options>) -> Result<()> {
    if let Some(file_name) = input.file_stem() {
        let file_name = file_name.to_str().unwrap();

        let extension = {
            if let Some(extension) = input.extension() {
                ".".to_string() + extension.to_str().unwrap()
            } else {
                String::from("")
            }
        };
        let full_file = file_name.to_string() + &extension;

        let new_file_name = detoxer.detox_file_name(file_name) + &extension;
        if new_file_name != full_file {
            let mut new_file = input.clone();
            new_file.set_file_name(&new_file_name);

            // Don't override existing file
            if !new_file.exists() {
                if options.verbose {
                    println!(
                        "\"{}\" -> \"{}\"",
                        input.to_str().unwrap(),
                        new_file.to_str().unwrap()
                    );
                }

                if !options.dry_run {
                    std::fs::rename(input, new_file)?;
                }
            } else {
                if let Some(move_to) = &options.move_to {
                    println!(
                        "Moving \"{}\" since \"{}\" already exists.",
                        input.to_str().unwrap(),
                        new_file.to_str().unwrap()
                    );

                    if !options.dry_run {
                        move_file_to(&input, &move_to)?;
                    }
                } else {
                    println!(
                        "Failed to rename \"{}\" since \"{}\" already exists.",
                        input.to_str().unwrap(),
                        new_file.to_str().unwrap()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Thread safe version for folder detoxing
pub fn detox(inputs: Arc<Mutex<Vec<PathBuf>>>, options: Arc<Options>) -> Result<()> {
    let detoxer = Detoxer::new()?;

    while let Some(input) = {
        let mut inputs = inputs.lock().unwrap();
        inputs.pop()
    } {
        if !hidden(&input) || options.hidden {
            if input.is_dir() {
                let new_inputs = fs::read_dir(&input)?
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .collect::<Vec<_>>();

                {
                    let mut inputs = inputs.lock().unwrap();
                    inputs.extend(new_inputs);
                }
            } else if input.is_file() {
                detox_file(input, &detoxer, &options)?;
            }
        }
    }

    Ok(())
}
