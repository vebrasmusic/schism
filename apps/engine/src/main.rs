mod cli;
mod environment;
mod temporal;

use cli::Cli;

fn main() {
    Cli::run();
}
