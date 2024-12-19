use crate::commands::uniswap_v3::contracts::Pool;
use crate::commands::Command;
use crate::commons::erc20::Erc20;
use crate::commons::helpers::{set_eth_balance, AlloyCacheDB};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_provider::Provider;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_rpc_types::Block;
use alloy_transport_http::Http;
use anyhow::Result;
use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgMatches};
use lazy_static::lazy_static;
use reqwest::Client;
use revm::db::{AlloyDB, CacheDB};
use revm::primitives::{address, bytes, Address, Bytecode, U256};
use std::str::FromStr;

// This bytecode correspond to the Uniswap V3 simulator contract at
// `@contracts/UniswapV3Simulator.sol`
lazy_static! {
    pub static ref UNISWAP_V3_SIMULATOR_CODE: Bytecode =
        Bytecode::new_raw(bytes!("608060405234801561000f575f80fd5b506004361061003f575f3560e01c806313d21cdf1461004357806364d27b5a14610090578063fa461e33146100c3575b5f80fd5b6100566100513660046106ed565b6100d8565b604080516001600160a01b0395861681529385166020850152919093169082015262ffffff90911660608201526080015b60405180910390f35b6100a361009e36600461071c565b61026b565b604080519485526020850193909352918301526060820152608001610087565b6100d66100d1366004610791565b61043b565b005b5f805f80846001600160a01b0316630dfe16816040518163ffffffff1660e01b8152600401602060405180830381865afa158015610118573d5f803e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061013c919061080d565b9350846001600160a01b031663d21220a76040518163ffffffff1660e01b8152600401602060405180830381865afa15801561017a573d5f803e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061019e919061080d565b9250846001600160a01b031663c45a01556040518163ffffffff1660e01b8152600401602060405180830381865afa1580156101dc573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610200919061080d565b9150846001600160a01b031663ddca3f436040518163ffffffff1660e01b8152600401602060405180830381865afa15801561023e573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906102629190610828565b90509193509193565b6040516370a0823160e01b81526001600160a01b0386811660048301525f918291829182918916906370a0823190602401602060405180830381865afa1580156102b7573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906102db919061084a565b6040516370a0823160e01b81526001600160a01b038b81166004830152919550908816906370a0823190602401602060405180830381865afa158015610323573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610347919061084a565b9150610356898b8a8989610575565b50506040516370a0823160e01b81526001600160a01b038a811660048301528916906370a0823190602401602060405180830381865afa15801561039c573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906103c0919061084a565b6040516370a0823160e01b81526001600160a01b038b81166004830152919450908816906370a0823190602401602060405180830381865afa158015610408573d5f803e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061042c919061084a565b90509650965096509692505050565b5f82828080601f0160208091040260200160405190810160405280939291908181526020018383808284375f920182905250601485015194955089131592506104f59150505760405163a9059cbb60e01b8152336004820152602481018790526001600160a01b0382169063a9059cbb906044016020604051808303815f875af11580156104cb573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906104ef9190610861565b5061056d565b5f85131561056d5760405163a9059cbb60e01b8152336004820152602481018690526001600160a01b0382169063a9059cbb906044016020604051808303815f875af1158015610547573d5f803e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061056b9190610861565b505b505050505050565b60408051606085901b6bffffffffffffffffffffffff191660208201528151601481830301815260349091019091525f9081906001600160a01b03871663128acb08898787816105e3576105de600173fffd8963efd1fc6a506488495d951d5263988d26610890565b6105f3565b6105f36401000276a360016108b5565b866040518663ffffffff1660e01b8152600401610614959493929190610902565b60408051808303815f875af192505050801561064d575060408051601f3d908101601f1916820190925261064a91810190610947565b60015b6106c0573d80801561067a576040519150601f19603f3d011682016040523d82523d5f602084013e61067f565b606091505b50806040516020016106919190610969565b60408051601f198184030181529082905262461bcd60e51b82526106b79160040161099e565b60405180910390fd5b90935091506106cc9050565b9550959350505050565b6001600160a01b03811681146106ea575f80fd5b50565b5f602082840312156106fd575f80fd5b8135610708816106d6565b9392505050565b80151581146106ea575f80fd5b5f805f805f8060c08789031215610731575f80fd5b863561073c816106d6565b9550602087013561074c816106d6565b9450604087013561075c816106d6565b9350606087013561076c816106d6565b9250608087013561077c8161070f565b8092505060a087013590509295509295509295565b5f805f80606085870312156107a4575f80fd5b8435935060208501359250604085013567ffffffffffffffff8111156107c8575f80fd5b8501601f810187136107d8575f80fd5b803567ffffffffffffffff8111156107ee575f80fd5b8760208284010111156107ff575f80fd5b949793965060200194505050565b5f6020828403121561081d575f80fd5b8151610708816106d6565b5f60208284031215610838575f80fd5b815162ffffff81168114610708575f80fd5b5f6020828403121561085a575f80fd5b5051919050565b5f60208284031215610871575f80fd5b81516107088161070f565b634e487b7160e01b5f52601160045260245ffd5b6001600160a01b0382811682821603908111156108af576108af61087c565b92915050565b6001600160a01b0381811683821601908111156108af576108af61087c565b5f81518084528060208401602086015e5f602082860101526020601f19601f83011685010191505092915050565b6001600160a01b0386811682528515156020830152604082018590528316606082015260a0608082018190525f9061093c908301846108d4565b979650505050505050565b5f8060408385031215610958575f80fd5b505080516020909101519092909150565b7202aa724a9aba0a82fab19902932bb32b93a1d1606d1b81525f82518060208501601385015e5f920160130191825250919050565b602081525f61070860208301846108d456fea26469706673582212206e2d54252527c62b7aa42fea6a3ff39ba9a9b2c1a78d9f0f854f667aa460d79464736f6c634300081a0033"));
}

#[derive(Debug, Clone)]
pub(crate) struct SwapViaPoolConfig {
    pool: Address,
    token_in: Address,
    amount: U256,
    caller: Address,
}

impl SwapViaPoolConfig {
    pub fn from_args(caller: Address, args: &ArgMatches) -> Self {
        let pool = args
            .get_one::<String>("pool")
            .expect("Pool address is required");
        let pool = Address::from_str(pool).expect("Invalid pool in address");

        let token_in = args
            .get_one::<String>("token-in")
            .expect("Token in is required");
        let token_in = Address::from_str(token_in).expect("Invalid token in address");

        let amount = args
            .get_one::<String>("amount")
            .expect("Amount is required");
        let amount = U256::from_str(amount).expect("Invalid amount");

        Self {
            pool,
            token_in,
            amount,
            caller,
        }
    }
}

pub struct SwapViaPool;

impl SwapViaPool {
    fn deploy_simulator(&self, database: &mut AlloyCacheDB) -> Result<Address> {
        // For educational purposes it is ok to use a random address. However, in a real scenario
        // this should be the address of the real contract.
        let simulator_address = address!("1100000000000000000000000000000000000011");

        let account = database.load_account(simulator_address).unwrap();
        account.info.code = Some(UNISWAP_V3_SIMULATOR_CODE.clone());
        Ok(simulator_address)
    }

    fn simulate(
        &self,
        block: &Block,
        client: &RootProvider<Http<Client>>,
        swap_configuration: &SwapViaPoolConfig,
    ) -> Result<(U256, U256)> {
        let block_id = BlockId::Number(BlockNumberOrTag::Number(block.header.number));
        let mut database = CacheDB::new(AlloyDB::new(client, block_id).unwrap());

        // One of the key differences with regards to Uniswap V2 is that the transfer must be
        // copmpleted within the same transaction as part of the callbacl that the pool executes on
        // the caller. For that reason it is not possible to call the pool directly as we did with
        // Uniswap V2 pools.
        let simulator_address = self.deploy_simulator(&mut database)?;

        let pool = Pool::new(
            swap_configuration.caller,
            swap_configuration.pool,
            simulator_address,
        );
        let pool_data = pool.get_pool_data(&mut database)?;

        let (zero_for_one, address_token_in, address_token_out) =
            if pool_data.token_0 == swap_configuration.token_in {
                (true, pool_data.token_0, pool_data.token_1)
            } else {
                (false, pool_data.token_1, pool_data.token_0)
            };

        let token_in = Erc20::new(swap_configuration.caller, address_token_in);
        let token_out = Erc20::new(swap_configuration.caller, address_token_out);

        // Optional step for convenience. Extreme caution is advised when using this method
        // as this might lead to a simulation that does not match the real state of the blockchain.
        set_eth_balance(
            swap_configuration.caller,
            swap_configuration.amount,
            &mut database,
        );
        let _ = token_in.set_balance(
            swap_configuration.caller,
            swap_configuration.amount,
            &mut database,
        );

        // Save the balances before perdorming the swap so that we can validate that the swap
        // was successful.
        let balance_in_before = token_in
            .balance_of(swap_configuration.caller, &mut database)?
            .0;
        let balance_out_before = token_out
            .balance_of(swap_configuration.caller, &mut database)?
            .0;

        // Mandatory step: Since the swap is performed by the simulator on our behalf, we need to
        // either apprrove the simulator or transfer the assert to it. In this implementation the
        // second approach was chosen.
        let _ = token_in.transfer(simulator_address, swap_configuration.amount, &mut database)?;

        let result = pool.swap(
            address_token_in,
            address_token_out,
            zero_for_one,
            swap_configuration.amount,
            swap_configuration.caller,
            &mut database,
        )?;

        // In order to ensure the swap was successful, we need to check the balances of both tokens
        // before and after the swap.
        let balance_in_after = token_in
            .balance_of(swap_configuration.caller, &mut database)?
            .0;
        let balance_out_after = token_out
            .balance_of(swap_configuration.caller, &mut database)?
            .0;

        assert!(
            balance_in_before - swap_configuration.amount == balance_in_after,
            "The balance of token in does not match the expected output"
        );
        assert!(
            result.0.tokenOutBalanceAfter == balance_out_after,
            "The balance of token out after does not match the expected output"
        );

        // TODO: For now we are just returning the input and output amounts, which is fine and works.
        // However, that means we will have to build the transaction again to send it to builders.
        // An alternative approach would be to build the final transaction, simulate it and if ok
        // return the transaction ready to be sent.
        let amount_out = balance_out_after.saturating_sub(balance_out_before);
        Ok((swap_configuration.amount, amount_out))
    }
}

#[async_trait]
impl Command for SwapViaPool {
    fn create(&self) -> clap::Command {
        clap::Command::new("swap-via-pool-v3")
            .about("Swap tokens via the Uniswap V3 Pool")
            .long_flag("swap-via-pool-v3")
            .arg(
                Arg::new("rpc-url")
                    .long("rpc-url")
                    .action(ArgAction::Set)
                    .required(true)
                    .help("The RPC URL to connect to"),
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
            )
    }

    fn name(&self) -> String {
        "swap-via-pool-v3".to_owned()
    }

    async fn execute(&self, args: &ArgMatches) {
        let rpc_url = args
            .get_one::<String>("rpc-url")
            .expect("RPC URL is required");
        let rpc_url = rpc_url.parse().unwrap();

        let client = ProviderBuilder::new().on_http(rpc_url);
        let block = client
            .get_block_by_number(BlockNumberOrTag::Latest, true)
            .await
            .unwrap()
            .unwrap();

        // The caller must be the public address that will sign the transactions,
        // which implies this wallet must be funded. For the purposes of this example
        // we are using a random address that is funded when simulating.
        let caller = address!("FF3cF7b8582571095A2B05268A4E1BafBDAD060D");

        let swap_configuration = SwapViaPoolConfig::from_args(caller, args);

        // Step 1: Based on the discovery made with the router, we know simulate the swap hitting
        // the poool straight away.
        let result = self
            .simulate(&block, &client, &swap_configuration)
            .expect("Error running simulation for pool");

        // Step 3: Build the final transaction and send it to builders.
        // Step 4: Monitor the chain until we find our transaction in a block
        // WIP
    }
}
