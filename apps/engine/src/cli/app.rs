use clap::Parser;

/// Schism engine command line interface.
///
/// Add new arguments as fields on this struct.
#[derive(Parser, Debug)]
#[command(name = "schism", version, about = "Schism engine", long_about = None)]
pub struct Cli {}

impl Cli {
    /// Parse arguments from the environment and run the CLI.
    pub fn run() {
        let parsed_arguments = Cli::parse();
        parsed_arguments.execute();
    }

    /// Execute the parsed command.
    fn execute(&self) {
        println!("welcome to schism");
    }
}
