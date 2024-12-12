use clap::{Arg, ArgAction, Command};
use commands::{
    eth_subscriptions::subscribe_new_block_headers,
    uniswap_v2::{swap_via_pool, swap_via_router},
};

mod commands;
mod commons;

#[tokio::main]
async fn main() {
    let matches = Command::new("revm-demo")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("swap-via-router")
                .about("Swap tokens via the Uniswap V2 Router")
                .long_flag("swap-via-router")
                .arg(
                    Arg::new("rpc-url")
                        .long("rpc-url")
                        .action(ArgAction::Set)
                        .required(true)
                        .help("The RPC URL to connect to")
                        // This is a free Alchemy API key, it is strongly recommended to create your own
                        // to avoid being thtrottled.
                        .default_value(
                            "https://eth-mainnet.g.alchemy.com/v2/Sg0Hh6Bcv4Dfj2OcU4_6VePVPED-8-MD",
                        ),
                )
                .arg(
                    Arg::new("token-in")
                        .long("token-in")
                        .help("The token to swap from")
                        .required(true)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("token-out")
                        .long("token-out")
                        .help("The token to swap to")
                        .required(true)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount")
                        .long("amount")
                        .help("The amount of token in to swap")
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("swap-via-pool")
                .about("Swap tokens via the Uniswap V2 Pool")
                .long_flag("swap-via-pool")
                .arg(
                    Arg::new("rpc-url")
                        .long("rpc-url")
                        .action(ArgAction::Set)
                        .required(true)
                        .help("The RPC URL to connect to")
                        // This is a free Alchemy API key, it is strongly recommended to create your own
                        // to avoid being thtrottled.
                        .default_value(
                            "https://eth-mainnet.g.alchemy.com/v2/Sg0Hh6Bcv4Dfj2OcU4_6VePVPED-8-MD",
                        ),
                )
                .arg(
                    Arg::new("token-in")
                        .long("token-in")
                        .help("The token to swap from")
                        .required(true)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("pool")
                        .long("pool")
                        .help("The pool address")
                        .required(true)
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount")
                        .long("amount")
                        .help("The amount of token in to swap")
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("subscribe-new-block-headers")
                .about("Subscribe to new block headers")
                .long_flag("subscribe-new-block-headers")
                .arg(
                    Arg::new("ws-url")
                        .long("ws-url")
                        .required(true)
                        .action(ArgAction::Set)
                        .help("The WS URL to subscribe to")
                        // This is a free Alchemy API key, it is strongly recommended to create your own
                        // to avoid being thtrottled.
                        .default_value(
                            "wss://eth-mainnet.g.alchemy.com/v2/Sg0Hh6Bcv4Dfj2OcU4_6VePVPED-8-MD",
                        ),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("swap-via-router", submatches)) => swap_via_router::execute(submatches).await,
        Some(("swap-via-pool", submatches)) => swap_via_pool::execute(submatches).await,
        Some(("subscribe-new-block-headers", submatches)) => {
            subscribe_new_block_headers::execute(submatches).await
        }
        _ => {
            println!("No subcommand provided");
        }
    }
}
