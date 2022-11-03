use std::fs;

use config::Config;

mod checker;
mod config;

fn main() -> Result<(), anyhow::Error> {
    let config_path = "shinydir.toml";
    let config_contents = fs::read_to_string(config_path)?;

    let config: Config = toml::from_str(&config_contents)?;

    let checker = checker::from_config(&config, None)?;
    checker.run();

    Ok(())
}
