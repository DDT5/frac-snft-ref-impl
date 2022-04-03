use std::{
    any::Any, 
    ops::Mul, 
    collections::HashMap};

use cosmwasm_std::{
    InitResponse, HandleResponse, StdResult, StdError,
    Extern, HumanAddr, Uint128, Binary, Env, Api,
    testing::{
        mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier
    }, 
};

use fractionalizer::{
    contract as frc, 
    msg::{
        InitMsg as FrcInitMsg, HandleMsg as FrcHandleMsg,
    }, 
    // msg as frc_msg,
    state::{
        UploadedFtkn as FrcUploadedFtkn, 
    },
};

use ftoken as ft;
use ftoken::{
    msg as ft_msg, 
};

// use secret_toolkit::utils::space_pad;
use snip721_reference_impl as s721;

use snip20_reference_impl as s20;

use fsnft_utils::{
    UndrNftInfo, ContractInfo, FtokenInit, FtokenConf, extract_cosmos_msg, BidConf
}; 


/////////////////////////////////////////////////////////////////////////////////
// App
/////////////////////////////////////////////////////////////////////////////////

/// stores persistent blockchain variables to perform multi-contract tests
/// # Arguments
/// * `deps` - in particular, all Storage is persistent and shared between contracts
/// * `env` - `block.height` and `block.time` are persistent, but `message.sender` needs to be 
/// changed before each tx
/// * `addrs` - HashMap of addresses (contracts or users) to be stored for convenience
pub(crate) struct App {
    pub(crate) deps: Extern<MockStorage, MockApi, MockQuerier>,
    pub(crate) env: Env,
    pub(crate) addrs: HashMap<String, ContractInfo>,
}

impl App {
    /// Helper method to initialize App and add addresses in the `addrs` HashMap
    /// * SNIP20 ("sSCRT") contract: `s20_addr` and `s20_hash`
    /// * SNIP721 ("NFT") contract: `s721_addr` and `s721_hash`
    /// * Fractionalizer: `frc_addr` and `frc_hash`
    /// * ftoken: `ft_addr` and `ft_hash`
    /// * NFT depositor: `user0`
    /// * `user1`
    /// * `user2` 
    pub(crate) fn new() -> Self {
        let mut app = App::new_blank();
        app.init_addrs();
        app
    }

    /// Initialize an App's deps and env 
    pub(crate) fn new_blank() -> Self {
        let mut env = mock_env("not_set", &[]);
        env.block.height = 0;
        env.block.time = 0;
        Self {
            deps: mock_dependencies(20, &[]),
            env,
            addrs: HashMap::new(),
        }
    }
    
    /// Adds addresses to App.addrs
    pub(crate) fn init_addrs(&mut self) -> () {
        // SNIP20 contract address
        self.addrs.insert("s20".to_string(), ContractInfo { 
            code_hash: "s20_hash".to_string(), 
            address: HumanAddr("s20_addr".to_string())
        });
        // SNIP721 contract address
        self.addrs.insert("s721".to_string(), ContractInfo { 
            code_hash: "s721_hash".to_string(), 
            address: HumanAddr("s721_addr".to_string()) 
        });
        // Fractionalizer contract
        self.addrs.insert("frc".to_string(), ContractInfo { 
            code_hash: "frc_hash".to_string(), 
            address: HumanAddr("frc_addr".to_string()) 
        });
        // ftoken contract
        self.addrs.insert("ft".to_string(), ContractInfo { 
            code_hash: "ft_hash".to_string(), 
            address: HumanAddr("ft_addr".to_string()) 
        });
        // user0 ("nft depositor") (user address, so has no code hash)
        self.addrs.insert("user0".to_string(), ContractInfo { 
            code_hash: "".to_string(), 
            address: HumanAddr("user0".to_string()) 
        });
        // user1 (user address, with no code hash)
        self.addrs.insert("user1".to_string(), ContractInfo { 
            code_hash: "".to_string(), 
            address: HumanAddr("user1".to_string()) 
        });
        // user2 (user address, with no code hash)
        self.addrs.insert("user2".to_string(), ContractInfo { 
            code_hash: "".to_string(), 
            address: HumanAddr("user2".to_string()) 
        });
    }

    // helper function to access the `addrs` Hashmap with less code (returns `ConfractInfo`)
    pub(crate) fn get_addr(&self, key: &str) -> ContractInfo {
        self.addrs.get(&key.to_string()).unwrap().to_owned()
    }

    /// changes the `env` variable to a user-inputted sender, and a contract's address and code hash 
    /// * `sender` - a &str key of the `sender` variable in `env`
    /// * `contract` - the &str key of the contract variable in `env`
    pub(crate) fn change_env(&mut self, sender: &str, contract_key: &str) {
        self.env.message.sender = self.get_addr(sender).address;
        self.env.contract.address = self.get_addr(contract_key).address;
        self.env.contract_code_hash = self.get_addr(contract_key).code_hash;
    }
    
    /// Increments block height and block time (assuming 5 seconds per block)
    /// * `count` - number of blocks to increment
    pub(crate) fn next_block(&mut self, count: u64) -> () {
        self.env.block.height += count;
        self.env.block.time += 5.mul(count);
    }
}




/////////////////////////////////////////////////////////////////////////////////
// Basic helper functions (extracting msgs, reading balances,)
/////////////////////////////////////////////////////////////////////////////////

pub(crate) fn extract_error_msg<T: Any>(error: StdResult<T>) -> String {
    match error {
        Ok(_response) => panic!("Expected error, but had Ok response"),
        Err(err) => match err {
            StdError::GenericErr { msg, .. } => msg,
            _ => panic!("Unexpected error result {:?}", err),
        },
    }
}

pub(crate) fn extract_log(resp: &StdResult<HandleResponse>) -> String {
    match resp {
        Ok(response) => response.log[0].value.clone(),
        Err(_err) => "These are not the logs you are looking for".to_string(),
    }
}

pub(crate) fn ftoken_balance(app: &mut App, addr_key: &str) -> Uint128 {
    Uint128(ft::state::ReadonlyBalances::from_storage(&app.deps.storage).account_amount(
        &app.deps.api.canonical_address(&app.get_addr(addr_key).address).unwrap()
    ))
}

pub(crate) fn s20_balance(app: &mut App, addr_key: &str) -> Uint128 {
    Uint128(s20::state::ReadonlyBalances::from_storage(&app.deps.storage).account_amount(
        &app.deps.api.canonical_address(&app.get_addr(addr_key).address).unwrap()
    ))
}


/////////////////////////////////////////////////////////////////////////////////
// pub functions: helpers
/////////////////////////////////////////////////////////////////////////////////

/// Initializes contracts and mints an NFT
/// Contracts initialized
/// * `SNIP721` - the NFT contract
/// # Arguments
pub(crate) fn init_default(app: &mut App) {
    // instantiate snip20, snip721 and fract contracts
    // -----------------------------------------------------------------------------
    let _s20_init_result = s20_init_helper_default(app);
    let _s721_init_result = s721_init_helper_default(app);
    let _frc_init_result = frc_init_helper_default(app);

    // mint "MyNFT" and approve fractionalizer to transfer it
    let _mint_result = s721_mint_nft_and_approve(
        app,
        "MyNFT",
        "user0",
        "frc",
    );    
}

pub(crate) fn fractionalize_default(app: &mut App) {
    // set default variables
    let nft_info = UndrNftInfo {
        token_id: "MyNFT".to_string(),
        nft_contr: app.get_addr("s721"),
    };
    let handle_msg = FrcHandleMsg::Fractionalize {
        nft_info: nft_info.clone(),
        ftkn_init: FtokenInit {
            name: "myftoken".to_string(),
            symbol: "TKN".to_string(),
            supply: Uint128(100),
            decimals: 6u8,
            ftkn_conf: FtokenConf {
                bid_conf: BidConf {
                    bid_token: app.get_addr("s20"),
                    min_ftkn_bond_prd: 10u64,
                    bid_period: 100,
                    bid_vote_quorum: Uint128(1000),
                }
            },
        },
    };

    sim_fractionalize(
        app,
        handle_msg,
    ).unwrap();
}


/// user0 transfers N ftokens to user1 after fractionalizing. Then both stake a certain amount in ftoken contract
/// 
/// # Arguments
/// * `transfer_amount` - amount of ftokens to transfer. In u128, will be converted to Uint128 in this function
/// * `user_from_stake` - in u128, will be converted to Uint128 in this function
/// * `user_to_stake` - in u128, will be converted to Uint128 in this function
pub(crate) fn transfer_ftkn_and_stake(
    app: &mut App,
    user_from: &str,
    user_to: &str,
    transfer_amount: u128,
    user_from_stake: u128,
    user_to_stake: u128,
) -> StdResult<()> {
    // user0 transfers n ftokens to user1
    app.change_env(user_from, "ft");
    let msg = ft::msg::HandleMsg::Transfer { recipient: app.get_addr(user_to).address, amount: Uint128(transfer_amount), memo: None, padding: None };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;

    // user0 stakes ftokens
    let msg = ft::msg::HandleMsg::Stake { amount: Uint128(user_from_stake) };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;

    // user1 stakes ftokens
    app.change_env(user_to, "ft");
    let msg = ft::msg::HandleMsg::Stake { amount: Uint128(user_to_stake) };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;

    Ok(())
}


/////////////////////////////////////////////////////////////////////////////////
// Simulation functions - for functions with inter-contract callback msgs
/////////////////////////////////////////////////////////////////////////////////

/// Simulates calling "fractionalize" handle function, with the cross-contract calls 
pub(crate) fn sim_fractionalize(
    app: &mut App,
    handle_msg: FrcHandleMsg,
) -> StdResult<()> {
    // call fractionalize handle on fract contract -----------------------------------
    app.change_env("user0", "frc");
    
    let handle_resp = frc::handle(&mut app.deps, app.env.clone(), handle_msg).unwrap();
    // check there are two messages in the response
    assert_eq!(handle_resp.messages.len(), 2);

    // message0: SNIP721 successfully register received ----------------------------
    // todo!(), low prioritiy



    // message1: contract-to-contract call init function on ftoken contract --------
    app.change_env("frc", "ft");
    let msg: ft_msg::InitMsg = extract_cosmos_msg(&handle_resp.messages[1]).unwrap();
    let ft_init_resp = ft::contract::init(&mut app.deps, app.env.clone(), msg).unwrap();

    // check there are two messages in the response
    assert_eq!(ft_init_resp.messages.len(), 2);

    // message0: contract-to-contract call ftoken init response -> fractionalizer handle ---
    app.change_env("ft", "frc");
    let msg: FrcHandleMsg = extract_cosmos_msg(&ft_init_resp.messages[0]).unwrap(); 
    let handle_resp = frc::handle(&mut app.deps, app.env.clone(), msg).unwrap();

    // check there is one message in the response
    assert_eq!(handle_resp.messages.len(), 1);

    // message1: ftoken init response -> SNIP721 SetViewingKey  --------------------
    // todo!()


    // fractionalizer -> SNIP721 `SendNft` handle ----------------------------------
    app.change_env("frc", "s721");
    let msg: s721::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp = s721::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 0);

    Ok(())
}

/// Simulates calling `Bid` on ftoken contract, with the inter-contract messages
/// # Arguments
/// * `amount` - amount of ftokens to transfer. In u128, will be converted to Uint128 in this function
pub(crate) fn sim_bid(
    app: &mut App,
    amount: u128,
    sender: &str,
) -> StdResult<()> { 
    // first give allowance to ftoken contract to spend snip20 tokens
    app.change_env(sender, "s20");
    let msg = s20::msg::HandleMsg::IncreaseAllowance { 
        spender: app.get_addr("ft").address, 
        amount: Uint128(1_000_000), 
        expiration: None, 
        padding: None 
    };
    let s20_allowance_result =s20::contract::handle(&mut app.deps, app.env.clone(), msg);
    assert!(s20_allowance_result.is_ok());
    // assert only works the first time, since this *increases* allowance 
    // let resp_data: s20::msg::HandleAnswer = from_binary(&s20_allowance_result.unwrap().data.unwrap()).unwrap();
    // println!("{:?}", resp_data);
    // let resp_data_bin = s20_allowance_result.unwrap().data.unwrap().0;
    // let mut exp_resp_data_bin = to_binary( 
    //         &s20::msg::HandleAnswer::IncreaseAllowance { 
    //             spender: app.get_addr("ft").address, 
    //             owner: app.get_addr(sender).address, 
    //             allowance: Uint128(1_000_000) 
    //         }
    //     ).unwrap().0;
    // let exp_resp_data_bin = space_pad(&mut exp_resp_data_bin, 256usize);
    // assert_eq!(resp_data_bin, *exp_resp_data_bin);

    // user2 makes bid
    app.change_env(sender, "ft");
    let msg = ft::msg::HandleMsg::Bid { amount: Uint128(amount) };
    let handle_resp = ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 1);

    // message0: ftoken contract -> `SendFrom` to snip20 contract
    app.change_env("ft", "s20");
    let msg: s20::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp =s20::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 1);

    // message0: s20 contract -> `Receive` to ftoken contract
    app.change_env("s20", "ft");
    let msg: ft::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp =ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 0);
    
    Ok(())
}

pub(crate) fn sim_retrieve_bid(
    app: &mut App,
    bid_id: u32,
    sender: &str,
) -> StdResult<HandleResponse>{
    app.change_env(sender, "ft");
    let msg = ft::msg::HandleMsg::RetrieveBid { bid_id };
    let handle_resp = ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;
    assert_eq!(handle_resp.messages.len(), 1);

    // message0: ftoken contract -> `Send` to snip20 contract
    app.change_env("ft", "s20");
    let msg: s20::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp =s20::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 0);

    Ok(HandleResponse::default())
}

/// simulates `RetrieveNft` function on ftoken contract
pub(crate) fn sim_retrieve_nft(
    app: &mut App,
    bid_id: u32,
    sender: &str,
) -> StdResult<HandleResponse> {
    app.change_env(sender, "ft");
    let msg = ft::msg::HandleMsg::RetrieveNft { bid_id };
    let handle_resp = ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;
    assert_eq!(handle_resp.messages.len(), 1);

    // ftoken -> SNIP721 `SendNft` handle -----------------------------------------
    app.change_env("ft", "s721");
    let msg: s721::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp = s721::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 0);

    Ok(HandleResponse::default())
}

/// simulates `ClaimProceeds` function on ftoken contract
pub(crate) fn sim_claim_proceeds(
    app: &mut App,
    sender: &str,
) -> StdResult<HandleResponse> {
    app.change_env(sender, "ft");
    let msg = ft::msg::HandleMsg::ClaimProceeds { };
    let handle_resp = ft::contract::handle(&mut app.deps, app.env.clone(), msg)?;
    assert_eq!(handle_resp.messages.len(), 1);

    // message0: ftoken contract -> `Send` to snip20 contract
    app.change_env("ft", "s20");
    let msg: s20::msg::HandleMsg = extract_cosmos_msg(&handle_resp.messages[0]).unwrap();
    let handle_resp =s20::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    assert_eq!(handle_resp.messages.len(), 0);

    Ok(HandleResponse::default())
}


/////////////////////////////////////////////////////////////////////////////////
// Private helper functions
/////////////////////////////////////////////////////////////////////////////////

fn frc_init_helper_default(app: &mut App) -> StdResult<InitResponse> {
    app.change_env("user0", "frc");

    let init_msg = FrcInitMsg {
        uploaded_ftoken: FrcUploadedFtkn::default(),
        bid_token: ContractInfo::default(),
    };

    frc::init(&mut app.deps, app.env.clone(), init_msg)
}

fn s20_init_helper_default(
    app: &mut App,
) -> StdResult<InitResponse> {
    app.change_env("user0", "s20");

    let mut initial_balances: Vec<s20::msg::InitialBalance> = vec![];
    initial_balances.push(s20::msg::InitialBalance {
        address: app.get_addr("user0").address,
        amount: Uint128(5_000),
    });
    initial_balances.push(s20::msg::InitialBalance {
        address: app.get_addr("user1").address,
        amount: Uint128(5_000),
    });
    initial_balances.push(s20::msg::InitialBalance {
        address: app.get_addr("user2").address,
        amount: Uint128(5_000),
    });

    let init_msg = s20::msg::InitMsg {
        name: "sec-sec".to_string(),
        admin: Some(HumanAddr("admin".to_string())),
        symbol: "SSCRT".to_string(),
        decimals: 8,
        initial_balances: Some(initial_balances),
        prng_seed: Binary::from("lolz fun yay".as_bytes()),
        config: None,
    };

    let init_result = s20::contract::init(&mut app.deps, app.env.clone(), init_msg);
    assert!(init_result.is_ok());
    init_result
}

/// Inits SNIP721 contract. "user0" set as admin/minter
fn s721_init_helper_default(app: &mut App) -> StdResult<InitResponse> {
    app.change_env("user0", "s721");
    
    let init_msg = s721::msg::InitMsg {
        name: "NFT Contract".to_string(),
        symbol: "NFT".to_string(),
        admin: Some(HumanAddr("user0".to_string())),
        entropy: "We're going to need a bigger boat".to_string(),
        royalty_info: None,
        config: None,
        post_init_callback: None,
    };

    let init_result = s721::contract::init(&mut app.deps, app.env.clone(), init_msg);
    assert!(init_result.is_ok());
    init_result
}

/// S721 contract mints an NFT
/// Default should be "user0" address mints an NFT with id "MyNFT" 
/// * `token_id` - a &str which will be converted to a String
/// * `owner_key` - the &str key for the address stored in App
/// * `addr_approved_key` address to be granted transfer approval. The &str key for the address stored in App
fn s721_mint_nft_and_approve(
    app: &mut App, 
    token_id: &str, 
    owner_key: &str,
    addr_approved_key: &str,
) -> StdResult<()> {
    // minter or owner is "user0"
    let user0 = app.get_addr(&owner_key.to_string()).address;

    // change environment
    app.change_env("user0", "s721");

    // set public and private metadata 
    let pub_expect = Some(s721::token::Metadata {
        token_uri: None,
        extension: Some(s721::token::Extension {
            name: Some("MyNFT".to_string()),
            description: None,
            image: Some("uri".to_string()),
            ..s721::token::Extension::default()
        }),
    });
    let priv_expect = Some(s721::token::Metadata {
        token_uri: None,
        extension: Some(s721::token::Extension {
            name: Some("MyNFTpriv".to_string()),
            description: Some("Nifty".to_string()),
            image: Some("privuri".to_string()),
            ..s721::token::Extension::default()
        }),
    });

    // mint NFT
    let handle_msg = s721::msg::HandleMsg::MintNft {
        token_id: Some(token_id.to_string()),
        owner: Some(user0),
        public_metadata: pub_expect,
        private_metadata: priv_expect,
        royalty_info: None,
        serial_number: None,
        transferable: None,
        memo: Some("Mint it baby!".to_string()),
        padding: None,
    };
    let handle_result = s721::contract::handle(&mut app.deps, app.env.clone(), handle_msg);
    let minted = extract_log(&handle_result);
    assert!(minted.contains(token_id));

    // set whitelist approval
    let handle_msg = s721::msg::HandleMsg::SetWhitelistedApproval {
        address: app.get_addr(&addr_approved_key.to_string()).address,
        token_id: Some(token_id.to_string()),
        view_owner: Some(s721::msg::AccessLevel::All),
        view_private_metadata: None,
        transfer: Some(s721::msg::AccessLevel::ApproveToken),
        expires: None,
        padding: None,
    };

    let handle_result = s721::contract::handle(&mut app.deps, app.env.clone(), handle_msg);
    assert!(handle_result.is_ok());
    Ok(())
}


