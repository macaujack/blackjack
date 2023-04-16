use blackjack_drivers::parse_config_from_file;
use clap::Parser;

const DEFAULT_CONFIG_PATH: &str = "~/.blackjack.yml";

#[derive(Debug, Parser)]
#[command(author, about, long_about = None)]
struct CommandLineArgs {
    /// The path of the config file
    #[arg(short, long, default_value_t = String::from(DEFAULT_CONFIG_PATH))]
    config: String,
}

fn main() {
    let mut args = CommandLineArgs::parse();
    if args.config == DEFAULT_CONFIG_PATH {
        let home_dir = home::home_dir().expect("Cannot find home directory");
        let config_file_path = home_dir.join(".blackjack.yml");
        if !config_file_path.exists() {
            panic!("Config file not exists");
        }
        if config_file_path.is_dir() {
            panic!("This should be a path rather than a directory");
        }
        args.config = String::from(config_file_path.to_str().unwrap());
    }
    let args = args;

    let config = parse_config_from_file(&args.config);
    println!("{:#?}", config);
}
