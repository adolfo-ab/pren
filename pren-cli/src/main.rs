mod config;

use crate::config::initialize_storage;
use arboard::Clipboard;
use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::CompleteEnv;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use pren_core::prompt::Prompt;
use pren_core::storage::PromptStorage;
use std::collections::HashMap;
use std::error::Error;

fn prompt_names(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let storage = initialize_storage(std::env::var("PREN_STORAGE_PATH").ok());

    let prompts = storage.get_prompts();
    match prompts {
        Ok(prompts) => prompts
            .iter()
            .map(|prompt| CompletionCandidate::new(&prompt.name))
            .collect(),
        Err(_) => vec![CompletionCandidate::new("")],
    }
}

#[derive(Parser)]
#[command(
    name = "pren",
    alias = "pren",
    bin_name = "pren",
    author = "Adolfo AB, adolfo.ab@proton.me",
    version,
    about = "A prompt engine designed for reusability and composability",
    long_about = "A prompt engine designed for reusability and composability"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    // The storage path where pren prompts are stored
    #[arg(long, short = 'p')]
    storage_path: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        #[arg(short = 'n', long, value_hint = ValueHint::Other)]
        name: String,
        #[arg(short = 'c', long)]
        content: String,
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(short = 'o', long)]
        overwrite: bool,
    },
    Show {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
    },
    Get {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
        #[arg(short = 'a', long, value_parser = parse_key_val, value_delimiter = ',')]
        args: Vec<(String, String)>,
    },
    List,
    Delete {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
        #[arg(short = 'f', long, default_value = "false")]
        force: bool,
    },
    Info,
}

/// Parse a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

fn main() -> Result<(), Box<dyn Error>> {
    CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    let storage = initialize_storage(cli.storage_path);

    match cli.command {
        Commands::Add {
            name,
            content,
            tags,
            overwrite,
        } => {
            match storage.get_prompt(&name) {
                Ok(_p) => {
                    if !overwrite {
                        eprintln!(
                            "Error: Prompt '{}' already exists. Use --overwrite to replace it.",
                            name
                        );
                        return Err(format!("Prompt '{}' already exists", name).into());
                    }
                }
                Err(_) => {}
            };
            // Create the prompt using the new unified constructor
            let prompt = Prompt::new(name.to_string(), content.to_string(), tags.clone())?;
            Ok(storage.save_prompt(&prompt)?)
        },
        Commands::Show {
            name,
        } => match storage.get_prompt(&name) {
            Ok(prompt) => {
                println!("{}", prompt.content);
                Ok(())
            }
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        }
        Commands::Get {
            name,
            args: kv_args,
        } => match storage.get_prompt(&name) {
            Ok(prompt) => {
                let mut clipboard = Clipboard::new()?;
                let args_map: HashMap<String, String> = kv_args.iter().cloned().collect();
                let rendered_prompt = prompt.render(&args_map, &storage)?;
                println!("{}", rendered_prompt);
                clipboard.set_text(rendered_prompt)?;
                Ok(())
            }
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        },
        Commands::List => {
            let prompts = storage.get_prompts();
            match prompts {
                Ok(p) => {
                    for prompt in p {
                        println!("Prompt name: {}", prompt.name);
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error retrieving prompts: '{}'", e);
                    Err(e.into())
                }
            }
        }
        Commands::Delete { name, force } => match storage.get_prompt(&name) {
            Ok(_prompt) => {
                if !force {
                    println!("Are you sure you want to delete prompt '{}'? [y/N]", name);
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    let input = input.trim().to_lowercase();
                    if input != "y" && input != "yes" {
                        println!("Delete operation cancelled.");
                        return Ok(());
                    }
                }

                match storage.delete_prompt(&name) {
                    Ok(()) => {
                        println!("Prompt '{}' deleted successfully.", name);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error deleting prompt '{}': {}", name, e);
                        Err(e.into())
                    }
                }
            },
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        },
        Commands::Info => {
            println!("Prompt storage path: {:?}", storage.base_path);
            println!("Total number of prompts: {}", storage.get_prompts().unwrap().len());
            Ok(())
        }
    }
}
