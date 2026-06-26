mod cli;

use cli::Cli;

fn main() -> anyhow::Result<()> {
    Cli::run()
}
