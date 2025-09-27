mod config;
mod constants;

use crate::config::{get_storage, PrenCliConfig};
use arboard::Clipboard;
use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::CompleteEnv;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use pren_core::prompt::{Prompt, PromptMetadata, PromptTemplate};
use pren_core::storage::PromptStorage;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use confy::ConfyError;
use pren_core::llm::get_completions_content;
use crate::constants::PREN_CLI;

// Custom completer for prompt names
fn prompt_names(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let storage = get_storage();

    let prompts = storage.get_prompts();
    match prompts {
        Ok(prompts) => prompts
            .iter()
            .map(|prompt| CompletionCandidate::new(&prompt.metadata.name))
            .collect(),
        Err(_) => vec![CompletionCandidate::new("")],
    }
}

// Custom completer for template arguments
fn prompt_args(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();

    // Parse the current input to extract the prompt name from previous args
    let args_strings: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = args_strings.iter().map(|s| s.as_str()).collect();

    // Find the prompt name from the --name argument
    let mut prompt_name = None;
    for i in 0..args.len() {
        if (args[i] == "-n" || args[i] == "--name") && i + 1 < args.len() {
            prompt_name = Some(args[i + 1]);
            break;
        }
    }

    let Some(name) = prompt_name else {
        return vec![CompletionCandidate::new("")];
    };

    // Get the prompt and extract its variables
    let storage = get_storage();
    let Ok(prompt) = storage.get_prompt(name) else {
        return vec![CompletionCandidate::new("")];
    };

    let prompt_args = match PromptTemplate::new(prompt) {
        Ok(template) => template.arguments(),
        Err(_) => return vec![CompletionCandidate::new("")],
    };


    // Parse already provided arguments to avoid duplicates
    let mut provided_keys = HashSet::new();
    for arg in std::env::args() {
        if let Some((key, _)) = arg.split_once('=') {
            provided_keys.insert(key.to_string());
        }
    }

    // If user is typing a new argument
    if current_str.is_empty() || !current_str.contains('=') {
        return prompt_args
            .into_iter()
            .filter(|var| !provided_keys.contains(var))
            .map(|var| CompletionCandidate::new(format!("{}=", var)))
            .collect();
    }

    // If user is in the middle of typing key=value, provide the key suggestions
    if let Some((partial_key, _)) = current_str.split_once('=') {
        let partial_key_string = partial_key.to_string();
        if prompt_args.contains(&&partial_key_string) {
            return vec![CompletionCandidate::new(current_str.to_string())];
        }
    }

    vec![CompletionCandidate::new("")]
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
        #[arg(short = 'd', long, value_hint = ValueHint::Other)]
        description: Option<String>,
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(short = 'c', long)]
        content: String,
        #[arg(short = 'o', long)]
        overwrite: bool,
    },
    Show {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
    },
    Render {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
        #[arg(short = 'a', long, value_parser = parse_key_val, value_delimiter = ',', add = ArgValueCompleter::new(prompt_args))]
        args: Vec<(String, String)>,
        #[arg(short = 'c', long)]
        copy: bool,
    },
    Get {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
        #[arg(short = 'a', long, value_parser = parse_key_val, value_delimiter = ',', add = ArgValueCompleter::new(prompt_args))]
        args: Vec<(String, String)>,
    },
    List,
    Delete {
        #[arg(short = 'n', long, add = ArgValueCompleter::new(prompt_names))]
        name: String,
        #[arg(short = 'f', long, default_value = "false")]
        force: bool,
    },
    Generate {
        #[arg(short = 'g', long, add = ArgValueCompleter::new(prompt_names))]
        generation_prompt: String,
        #[arg(short = 'a', long, value_parser = parse_key_val, value_delimiter = ',', add = ArgValueCompleter::new(prompt_args))]
        args: Vec<(String, String)>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config: PrenCliConfig = confy::load(PREN_CLI, None)?;

    CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    let storage = get_storage();

    match cli.command {
        Commands::Add {
            name,
            description,
            tags,
            content,
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
            Ok(storage.save_prompt(&Prompt::new(PromptMetadata::new(name,description,tags), content))?)
        }
        Commands::Show { name } => match storage.get_prompt(&name) {
            Ok(prompt) => {
                println!("Name: {}", prompt.metadata.name);
                println!("Tags: {:?}", prompt.metadata.tags);
                println!("Content:\n{}", prompt.content);
                Ok(())
            }
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        },
        Commands::Render {
            name,
            args: kv_args,
            copy,
        } => match storage.get_prompt(&name) {
            Ok(prompt) => {
                let args_map: HashMap<String, String> = kv_args.iter().cloned().collect();
                let rendered_prompt = PromptTemplate::new(prompt)?.render(&args_map, &storage)?;
                println!("{}", rendered_prompt);
                if copy {
                    Clipboard::new()?.set_text(rendered_prompt)?;
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        },
        Commands::Get {
            name,
            args: kv_args,
        } => match storage.get_prompt(&name) {
            Ok(prompt) => {
                let args_map: HashMap<String, String> = kv_args.iter().cloned().collect();
                let rendered_prompt = PromptTemplate::new(prompt)?.render(&args_map, &storage)?;
                Clipboard::new()?.set_text(rendered_prompt)?;
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
                        println!("Prompt name: {}", prompt.metadata.name);
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
            }
            Err(e) => {
                eprintln!("Error retrieving prompt '{}': {}", name, e);
                Err(e.into())
            }
        },
        Commands::Generate { generation_prompt, args } => {
            match storage.get_prompt(&generation_prompt) {
                Ok(prompt) => {
                    let args_map: HashMap<String, String> = args.iter().cloned().collect();
                    let rendered_prompt = PromptTemplate::new(prompt)?.render(&args_map, &storage)?;
                    let response = get_completions_content(&config.model_config.api_key, &config.model_config.base_url, &config.model_config.model_name, &rendered_prompt).await?;

                    println!("{}", response);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error generating prompt using '{}': {}", generation_prompt, e);
                    Err(e.into())
                }
            }
        },
        Commands::Info => {
            println!("Prompt storage path: {:?}", storage.base_path);
            println!(
                "Total number of prompts: {}",
                storage.get_prompts()?.len()
            );
            Ok(())
        },

    }
}
