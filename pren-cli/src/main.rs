mod config;

use crate::config::initialize_storage;
use clap::{Parser, Subcommand};
use std::error::Error;
use pren_core::file_storage;
use pren_core::file_storage::FileStorage;
use pren_core::prompt::Prompt;
use pren_core::registry::PromptStorage;

#[derive(Parser, Debug)]
#[command(version,
alias = "pren",
display_name = "pren",
bin_name = "pren",
author="Adolfo AB,\
 adolfo.ab@proton.me",
about="A simple and ergonomic prompt engine",
long_about="A simple and ergonomic prompt engine", )]
struct Args {
    #[arg(short = 'p', long)]
    storage_path: Option<String>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Add {
        #[arg(short = 'n', long)]
        name: String,
        #[arg(short = 'c', long)]
        content: String,
        #[arg(short = 't', long)]
        tags: Vec<String>,
        #[arg(short = 's', long)]
        prompt_type: String,
        #[arg(short = 'o', long)]
        overwrite: bool,
    },
    Get {
        #[arg(short = 'n', long)]
        name: String,
        #[arg(short = 'a', long)]
        args: Vec<String>,
        #[arg(short = 'c', long)]
        copy: bool,
    },
    List,
    Delete {
        #[arg(short = 'n', long)]
        name: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let _storage = initialize_storage(args.storage_path);

    match &args.cmd {
        Commands::Add { name, content, prompt_type,  tags, overwrite} => {
            match _storage.get_prompt(name) {
                Ok(Some(p)) => {
                    if !*overwrite {
                         return Ok(());
                    }
                }
                _ => {},
            };
            match prompt_type.as_str() {
                "simple" => Ok(_storage.save_prompt(&Prompt::new_simple(name.to_string(), content.to_string(), tags.clone()))?),
                "template" => Ok(_storage.save_prompt(&Prompt::new_simple(name.to_string(), content.to_string(), tags.clone()))?),
                _ => Err("Invalid prompt type, must be 'simple' or 'template'".into())
            }
        }
        _ => Ok(()),
    }
}
