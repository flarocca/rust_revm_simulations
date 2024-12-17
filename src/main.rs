mod commands;
mod commons;

#[tokio::main]
async fn main() {
    let commands = commands::get_commands();

    let mut clap_commands = clap::Command::new("revm-demo")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true);

    for command in commands.values() {
        clap_commands = clap_commands.subcommand(command.create());
    }

    let matches = clap_commands.get_matches();
    match matches.subcommand() {
        Some(subcommand) => {
            let (subcommand_name, subcommand_args) = subcommand;
            let command = commands.get(subcommand_name).unwrap();
            command.execute(subcommand_args).await;
        }
        _ => {
            println!("No subcommand provided");
        }
    }
}
