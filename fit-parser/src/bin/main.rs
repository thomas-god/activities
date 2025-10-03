use clap::Parser;
use fit_parser::parse_fit_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_cli()
}

#[derive(Parser, Debug)]
#[command(name = "fit-parser-cli")]
#[command(about = "A CLI wrapper for parsing .fit files")]
#[command(next_line_help = true)]
struct Cli {
    #[arg(long)]
    file: String,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let file = cli.file;

    let messages = parse_fit_file(&file, false).unwrap();

    for message in messages {
        println!("{message:?}");
    }

    Ok(())
}
