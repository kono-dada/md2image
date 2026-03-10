use clap::Parser;
use md2image::{Cli, run};

fn main() {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(error.exit_code().into());
        }
    }
}
