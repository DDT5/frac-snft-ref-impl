use cosmwasm_std::{
    HumanAddr, Uint128, to_binary, CosmosMsg, WasmMsg,
    testing::{
        mock_env,
    }, 
    Api,
};

use secret_toolkit::utils::{HandleCallback};

use cosmwasm_storage::ReadonlyPrefixedStorage;

// use fractionalizer as frc; 
use fractionalizer::{
    state::{
        ftoken_instance_r,
    },
};

use ftoken as ft;
use ftoken::{
    ftoken_mod::{
        state::*, 
    }
};

use snip721_reference_impl as s721;
use snip721_reference_impl::{
    state::{
        PREFIX_INFOS,
        json_load,
    }
};

// use snip20_reference_impl as s20;

use fsnft_utils::{
    UndrNftInfo, FtokenInfo, FtokenInstance, extract_cosmos_msg, InterContrMsg, BidStatus, BidsInfo, 
};

use crate::helpers::{
    App, extract_error_msg,
    init_default, fractionalize_default, ftoken_balance, s20_balance, sim_bid, transfer_ftkn_and_stake, sim_retrieve_nft, 
    sim_retrieve_bid, sim_claim_proceeds,
};


/////////////////////////////////////////////////////////////////////////////////
// Multi-contract unit tests
/////////////////////////////////////////////////////////////////////////////////

#[test]
fn init_default_sanity() {
    let mut app = App::new();
    init_default(&mut app);
    
    // assert user0 has nft
    let token: s721::token::Token = json_load(
        &ReadonlyPrefixedStorage::new(PREFIX_INFOS, &app.deps.storage), &0u32.to_le_bytes()
    ).unwrap();
    assert_eq!(app.deps.api.human_address(&token.owner).unwrap(), app.get_addr("user0").address);

    // assert users have correct initial token balances
    let users = vec!["user0", "user1", "user2"];
    let mut user_s20_bal: Vec<Uint128> = vec![];
    let mut user_ftoken_bal: Vec<Uint128> = vec![];
    for user in users.iter() {
        user_s20_bal.push(s20_balance(&mut app, user));
        user_ftoken_bal.push(ftoken_balance(&mut app, user))
    }
    // assert s20 balances
    assert_eq!(user_s20_bal, vec![Uint128(5_000); 3]);
    // assert ftoken balances
    assert_eq!(user_ftoken_bal, vec![Uint128(0); 3]);
}

#[test]
fn frac_default_sanity() {
    // init contracts, create NFT and give approval to frc contract ----------------
    let mut app = App::new();
    init_default(&mut app);

    // fractionalize NFT -----------------------------------------------------------
    fractionalize_default(&mut app);
    // initial ftoken balance is correct -- that NFT depositor receives the "supply" amount of ftokens
    let depositor_bal = ftoken_balance(&mut app, "user0");
    assert_eq!(depositor_bal, Uint128(100));

    // ftkn_info stored in fractionalizer contract == stored in ftoken contract 
    let frc_ftkn_instance = ftoken_instance_r(&app.deps.storage).load(&0u32.to_le_bytes()).unwrap();
    let ft_ftkn_info = ftoken_info_r(&app.deps.storage).load().unwrap();
    assert_eq!(&frc_ftkn_instance, &ft_ftkn_info.instance);

    // check that ftokenInfo stored correctly in ftoken contract
    let exp_ft_ftkn_info = FtokenInfo {
        instance: FtokenInstance {
            ftkn_idx: 0u32,
            depositor: app.get_addr("user0").address,
            ftoken_contr: app.get_addr("ft"),
            init_nft_info: UndrNftInfo {
                token_id: "MyNFT".to_string(),
                nft_contr: app.get_addr("s721"),
            },
            name: "myftoken".to_string(),
            symbol: "TKN".to_string(),
            decimals: 6u8,
        },
        in_vault: true,
    };
    assert_eq!(ft_ftkn_info, exp_ft_ftkn_info);

    // NFT is now in ft contract
    let token: s721::token::Token = json_load(
        &ReadonlyPrefixedStorage::new(PREFIX_INFOS, &app.deps.storage), &0u32.to_le_bytes()
    ).unwrap();
    assert_eq!(app.deps.api.human_address(&token.owner).unwrap(), app.get_addr("ft").address);
}   

#[test]
fn test_stake_unstake() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);

    // user with no ftokens try to stake, should not work
    app.change_env("user1", "ft");
    let msg = ft::msg::HandleMsg::Stake { amount: Uint128(100) };
    let handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    let error = extract_error_msg(handle_result);
    assert!(error.contains("insufficient funds"));

    // stake 200 ftokens, more than available, should not work
    app.change_env("user0", "ft");
    let msg = ft::msg::HandleMsg::Stake { amount: Uint128(200) };
    let handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    let error = extract_error_msg(handle_result);
    assert!(error.contains("insufficient funds"));

    // stake 100, should work
    app.change_env("user0", "ft");
    let msg = ft::msg::HandleMsg::Stake { amount: Uint128(100) };
    let _handle_resp = ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    let mut exp_staked = StakedTokens {
        amount: Uint128(100),
        unlock_height: 10u64,
    };
    let act_staked = ftkn_stake_r(&app.deps.storage).load(
        &to_binary(&app.get_addr("user0").address).unwrap().as_slice()
    ).unwrap();
    // check 100 ftoken staked, bonded for 10 blocks 
    assert_eq!(act_staked, exp_staked);  
    // check depositor has 0 ftokens
    let depositor_bal = ftoken_balance(&mut app, "user0");
    assert_eq!(depositor_bal, Uint128(0));

    app.next_block(5);
    let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(100) };
    let handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    let error = extract_error_msg(handle_result);
    assert!(error.contains("ftokens are still bonded. Will unbond at height 10"));

    app.next_block(5);
    let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(500) };
    let handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg.clone());
    let error = extract_error_msg(handle_result);
    assert!(error.contains("insufficient funds"));

    app.change_env("user1", "ft");
    let handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    let error = extract_error_msg(handle_result);
    assert!(error.contains("this address has not staked ftokens before"));

    // unstake partially should work
    app.change_env("user0", "ft");
    let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(20) };
    let _handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    exp_staked.amount = Uint128(80);
    let act_staked = ftkn_stake_r(&app.deps.storage).load(
        &to_binary(&app.get_addr("user0").address).unwrap().as_slice()
    ).unwrap();
    // check 80 ftoken staked now
    assert_eq!(act_staked, exp_staked);  
    // check depositor has 20 ftokens
    let depositor_bal = ftoken_balance(&mut app, "user0");
    assert_eq!(depositor_bal, Uint128(20));

    // unstake remaining ftokens after a while, should work
    app.next_block(100);
    let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(80) };
    let _handle_result = ft::contract::handle(&mut app.deps, app.env.clone(), msg);
    exp_staked.amount = Uint128(0);
    let act_staked = ftkn_stake_r(&app.deps.storage).load(
        &to_binary(&app.get_addr("user0").address).unwrap().as_slice()
    ).unwrap();
    // check 0 ftoken staked now
    assert_eq!(act_staked, exp_staked);  
    // check depositor has all 100 ftokens back
    let depositor_bal = ftoken_balance(&mut app, "user0");
    assert_eq!(depositor_bal, Uint128(100));
}

#[test]
fn test_transfer_ftokens_sanity() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);
    let handle_result = transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 0, 0);
    
    // user0 transfers 30 ftokens to user1
    assert!(handle_result.is_ok());
    let user0_bal = ftoken_balance(&mut app, "user0");
    let user1_bal = ftoken_balance(&mut app, "user1");
    assert_eq!(user0_bal, Uint128(70)); 
    assert_eq!(user1_bal, Uint128(30));

    // user0 transfers 200 ftokens to user1, should not work
    let handle_result = transfer_ftkn_and_stake(&mut app, "user0", "user1", 200, 0, 0);
    let error = extract_error_msg(handle_result);
    assert!(error.contains("insufficient funds")); 
}

#[test]
fn test_bidding_retrievenft_forced() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);
    sim_bid(&mut app, 2_000, "user2").unwrap();
    transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 0, 0).unwrap();

    // check bid_info is correct
    let mut exp_bid_info = BidsInfo {
        bid_id: 0u32,
        bidder: app.get_addr("user2").address,
        amount: Uint128(2_000),
        status: BidStatus::Active,
        end_height: 100u64,
    };
    let mut bid_info_0 = bids_r(&app.deps.storage).load(&0u32.to_le_bytes()).unwrap();
    assert_eq!(bid_info_0, exp_bid_info);
    assert_eq!(Uint128(3_000), s20_balance(&mut app, "user2"));

    // user2 can make another bid
    sim_bid(&mut app, 3_000, "user2").unwrap();
    exp_bid_info.bid_id = 1u32;
    exp_bid_info.amount = Uint128(3_000);
    let mut bid_info_1 = bids_r(&app.deps.storage).load(&1u32.to_le_bytes()).unwrap();
    assert_eq!(bid_info_1, exp_bid_info);
    // user2 now has zero s20 (sscrt) token balance
    assert_eq!(Uint128(0), s20_balance(&mut app, "user2"));

    // user2 cannot retrieve nft
    let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
    let error = extract_error_msg(handle_result);
    assert!(error.contains("Cannot retrieve underlying NFT: bid status is not `WonInVault`"));

    // forcefully change bid_status of bid_id: 0. user2 cannot `RetrieveNft`
    let status_vec = vec![BidStatus::WonRetrieved, BidStatus::Active, BidStatus::LostInTreasury, BidStatus::LostRetrieved];
    for status in status_vec.iter() {
        bid_info_0.status = status.clone();
        bids_w(&mut app.deps.storage).save(&0u32.to_le_bytes(), &bid_info_0).unwrap();
        let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
        let error = extract_error_msg(handle_result);
        assert!(error.contains("Cannot retrieve underlying NFT: bid status is not `WonInVault`"));
    }
    
    // forcefully change bid_status of bid_id: 0 to `WonInVault`, and `won_bid_id` = 0:
    won_bid_id_w(&mut app.deps.storage).save(&0u32).unwrap(); 
    // other (user1) cannot retrieve...
    bid_info_0.status = BidStatus::WonInVault;
    bids_w(&mut app.deps.storage).save(&0u32.to_le_bytes(), &bid_info_0).unwrap();
    let handle_result = sim_retrieve_nft(&mut app, 0u32, "user1");
    let error = extract_error_msg(handle_result);
    assert!(error.contains("Cannot retrieve underlying NFT: You are not the bidder"));

    // ...user2 can retrieve nft
    let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
    assert!(handle_result.is_ok());
    // user2 now has NFT
    let token: s721::token::Token = json_load(
        &ReadonlyPrefixedStorage::new(PREFIX_INFOS, &app.deps.storage), &0u32.to_le_bytes()
    ).unwrap();
    assert_eq!(app.deps.api.human_address(&token.owner).unwrap(), app.get_addr("user2").address);
    // user2 still has zero s20 (sscrt) token balance
    assert_eq!(Uint128(0), s20_balance(&mut app, "user2"));

    // forcefully change bid_status of bid_id: 1. user2 cannot `RetrieveBid`
    let status_vec = vec![BidStatus::WonRetrieved, BidStatus::WonInVault, BidStatus::Active, BidStatus::LostRetrieved];
    for status in status_vec.iter() {
        bid_info_1.status = status.clone();
        bids_w(&mut app.deps.storage).save(&1u32.to_le_bytes(), &bid_info_1).unwrap();
        let handle_result = sim_retrieve_bid(&mut app, 1u32, "user2");
        let error = extract_error_msg(handle_result);
        assert!(error.contains("Cannot retrieve bid tokens: bid status is not `LostInTreasury`"));
    }
    // forcefully change bid_status of bid_id: 1 to `LostInTreasury`: 
    // other (user1) cannot retrieve...
    bid_info_1.status = BidStatus::LostInTreasury;
    bids_w(&mut app.deps.storage).save(&1u32.to_le_bytes(), &bid_info_1).unwrap();
    let handle_result = sim_retrieve_bid(&mut app, 1u32, "user1");
    let error = extract_error_msg(handle_result);
    assert!(error.contains("Cannot retrieve bid tokens: You are not the bidder"));
    
    // ...user2 can retrieve bid
    let handle_result = sim_retrieve_bid(&mut app, 1u32, "user2");
    assert!(handle_result.is_ok());
    // user2 now has 3000 s20 (sscrt) token balance
    assert_eq!(Uint128(3_000), s20_balance(&mut app, "user2"));

    // user0 and user1 can claim pro-rata sale proceeds 
    assert_eq!(Uint128(5_000), s20_balance(&mut app, "user0"));
    assert_eq!(Uint128(5_000), s20_balance(&mut app, "user1"));
    sim_claim_proceeds(&mut app, "user0").unwrap();
    sim_claim_proceeds(&mut app, "user1").unwrap();
    assert_eq!(Uint128(5_000 + 70*2_000/100), s20_balance(&mut app, "user0"));
    assert_eq!(Uint128(5_000 + 30*2_000/100), s20_balance(&mut app, "user1"));
}

#[test]
fn test_bid_votes() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);
    transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 70, 30).unwrap();
    sim_bid(&mut app, 2_000, "user2").unwrap();
    // todo

    // // unstake ftokens
    // app.change_env("user0", "ft");
    // let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(70) };
    // ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();

    // app.change_env("user1", "ft");
    // let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(30) };
    // ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();

    
}

/////////////////////////////////////////////////////////////////////////////////
// Misc tests
/////////////////////////////////////////////////////////////////////////////////

#[test]
fn test_decode() {
    let env = mock_env("sender", &[]);
    let msg0 = to_binary(&InterContrMsg::register_receive(&env.contract_code_hash)).unwrap();
    let message0: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
    contract_addr: HumanAddr("nft_addr".to_string()),
    callback_code_hash: "nft_hash".to_string(),
    msg: msg0,
    send: vec![],
    });

    let _decoded0: InterContrMsg = extract_cosmos_msg(&message0).unwrap();
    // println!("The decoded CosmosMsg0 is: {:?}", decoded0);    

    let msg1 = InterContrMsg::register_receive(&env.contract_code_hash);
    let _message1 = msg1.to_cosmos_msg(
        "nft_hash".to_string(),
        HumanAddr("nft_addr".to_string()),
        None
    );
    // // let decoded1: InterContrMsg = extract_cosmos_msg(&message1.unwrap()).unwrap();
    // println!("The decoded CosmosMsg1 is: {:?}", extract_cosmos_msg::<InterContrMsg>(&message1.unwrap()).unwrap());
}

