use alloy_signer_local::PrivateKeySigner;
use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgMatches};
use revm::primitives::{keccak256, Address};

use crate::commands::Command;

pub struct ComputeAddress;

#[async_trait]
impl Command for ComputeAddress {
    fn create(&self) -> clap::Command {
        clap::Command::new("compute-address-from-private-key")
            .about("Compute the address from a private key")
            .long_flag("compute-address-from-private-key")
            .arg(
                Arg::new("private-keys")
                    .long("private-keys")
                    .required(true)
                    .action(ArgAction::Set)
                    .help("The list of private keys to derive")
                    .num_args(1..)
                    .value_delimiter(','.to_owned()),
            )
    }

    fn name(&self) -> String {
        "compute-address-from-private-key".to_owned()
    }

    async fn execute(&self, args: &ArgMatches) {
        let private_keys = args
            .get_many::<String>("private-keys")
            .expect("At least one private key is required");

        for private_key in private_keys {
            let signer = &private_key.parse::<PrivateKeySigner>().unwrap();
            let public_key = &signer
                .clone()
                .into_credential()
                .verifying_key()
                .to_encoded_point(false);

            let public_key_bytes = &public_key.as_bytes()[1..];

            let public_key_hash = keccak256(public_key_bytes);
            let address = &public_key_hash[12..];
            let address = Address::from_slice(address);

            println!("Address: {:#?}", address);
        }
    }
}
