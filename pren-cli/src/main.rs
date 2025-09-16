mod config;

use arboard::Clipboard;
use crate::config::initialize_storage;
use std::collections::HashMap;
use std::error::Error;
use pren_core::prompt::Prompt;
use pren_core::registry::PromptStorage;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::CompleteEnv;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};


fn prompt_names(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let storage = initialize_storage(std::env::var("PREN_STORAGE_PATH").ok());

    let prompts = storage.get_prompts();
    match prompts {
        Ok(prompts) => prompts.iter().map(|prompt| CompletionCandidate::new(prompt.name())).collect(),
        Err(_) => vec![CompletionCandidate::new("")]
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
    long_about = "A prompt engine designed for reusability and composability",
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    // The storage path where pren prompts are stored
    #[arg(long, short= 'p')]
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
        #[arg(short = 's', long, value_parser=["simple", "template"])]
        prompt_type: String,
        #[arg(short = 'o', long)]
        overwrite: bool,
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
    let mut cli = Cli::parse();
    let _storage = initialize_storage(cli.storage_path);

    match cli.command {
        Commands::Add { name, content, prompt_type,  tags, overwrite} => {
            match _storage.get_prompt(&name) {
                Ok(_p) => {
                    if !overwrite {
                        eprintln!("Error: Prompt '{}' already exists. Use --overwrite to replace it.", name);
                        return Err(format!("Prompt '{}' already exists", name).into());
                    }
                }
                Err(_) => {},
            };
            match prompt_type.as_str() {
                "simple" => Ok(_storage.save_prompt(&Prompt::new_simple(name.to_string(), content.to_string(), tags.clone()))?),
                "template" => Ok(_storage.save_prompt(&Prompt::new_template(name.to_string(), content.to_string(), tags.clone())?)?),
                _ => Err("Invalid prompt type, must be 'simple' or 'template'".into())
            }
        }
        Commands::Get { name, args: kv_args } => {
            match _storage.get_prompt(&name) {
                Ok(prompt) => {
                    let mut clipboard = Clipboard::new()?;
                    let args_map: HashMap<String, String> = kv_args.iter().cloned().collect();
                    let rendered_prompt = prompt.render(&args_map, &_storage)?;
                    println!("{}", rendered_prompt);
                    clipboard.set_text(rendered_prompt)?;
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error retrieving prompt '{}': {}", name, e);
                    Err(e.into())
                }
            }
        }
        Commands::List => {
            let prompts = _storage.get_prompts();
            match prompts {
                Ok(p) => {
                    for prompt in p {
                        println!("Prompt name: {}", prompt.name());
                    }
                    Ok(())
                },
                Err(e) => {
                    eprintln!("Error retrieving prompts: '{}'", e);
                    Err(e.into())
                }
            }
        }
        Commands::Delete { name, force } => {
            match _storage.get_prompt(&name) {
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
                    
                    match _storage.delete_prompt(&name) {
                        Ok(()) => {
                            println!("Prompt '{}' deleted successfully.", name);
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Error deleting prompt '{}': {}", name, e);
                            Err(e.into())
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error retrieving prompt '{}': {}", name, e);
                    Err(e.into())
                }
            }
        },

    }
}
