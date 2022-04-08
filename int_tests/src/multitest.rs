use cosmwasm_std::{
    Uint128, to_binary, 
    Api,
};

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
    UndrNftInfo, FtokenInfo, FtokenInstance, AucConf, // FtokenInit, FtokenConf, AucConf, PropConf,
};

use crate::helpers::{
    App, extract_error_msg,
    init_default, fractionalize_default, ftoken_balance, s20_balance, transfer_ftkn_and_stake, sim_bid, 
    sim_finalize_auction, sim_retrieve_bid, sim_claim_proceeds,
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
        vault_active: true,
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
fn test_auction_process() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);
    transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 70, 30).unwrap();

    app.change_env("user1", "ft");
    let mut msg = ft::msg::HandleMsg::VoteReservationPrice { resv_price: Uint128(99) };
    let mut error = extract_error_msg(ft::contract::handle(&mut app.deps, app.env.clone(), msg));
    assert!(error.contains("Reserve price out of bounds. Please set between "));

    msg = ft::msg::HandleMsg::VoteReservationPrice { resv_price: Uint128(2501) };
    error = extract_error_msg(ft::contract::handle(&mut app.deps, app.env.clone(), msg));
    assert!(error.contains("Reserve price out of bounds. Please set between "));

    msg = ft::msg::HandleMsg::VoteReservationPrice { resv_price: Uint128(100) };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    let mut agg_resv_price = agg_resv_price_r(&app.deps.storage).load().unwrap();
    assert_eq!(agg_resv_price.uint128_price(), Uint128(100));

    // cannot bid because token not yet unlocked
    error = extract_error_msg(sim_bid(&mut app, 1000, None));
    assert!(error.contains("vault is not unlocked. Unlock threshold is "));

    // user0 votes on reservation price too
    // add blocks to later test that this tx should increase the bonding period of ftokens
    app.next_block(5);
    app.change_env("user0", "ft");
    msg = ft::msg::HandleMsg::VoteReservationPrice { resv_price: Uint128(50) };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    agg_resv_price = agg_resv_price_r(&app.deps.storage).load().unwrap();
    assert_eq!(agg_resv_price.uint128_price(), Uint128(65));

    // unstake tx changes reservation price, does not increase bonding period
    app.next_block(5);
    msg = ft::msg::HandleMsg::Unstake { amount: Uint128(40) };
    error = extract_error_msg(ft::contract::handle(&mut app.deps, app.env.clone(), msg.clone()));
    assert!(error.contains("ftokens are still bonded. Will unbond at height 15"));
    
    app.next_block(5);
    ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();
    agg_resv_price = agg_resv_price_r(&app.deps.storage).load().unwrap();
    assert_eq!(agg_resv_price.uint128_price(), Uint128(75));

    // bid below reservation price does not work
    app.change_env("user2", "ft");
    error = extract_error_msg(sim_bid(&mut app, 70, None));
    assert!(error.contains("bid must be equal or greater than the reservation price of "));
    
    // bid at reservation price works
    sim_bid(&mut app, 75, None).unwrap();
    let (mut bid, mut pos) = get_last_bid(&app.deps.storage).unwrap();
    let mut exp_bid = BidInfo { 
        bidder: app.get_addr("user2").address, amount: Uint128(75), winning_bid: false, retrieved_bid: false 
    };
    assert_eq!(bid, exp_bid);
    assert_eq!(pos, 0u32);
    let mut auc_status = auction_info_r(&app.deps.storage).load().unwrap();
    let exp_auc_status = AuctionInfo {
        is_active: true,
        end_height: 115u64,
        auc_config_snapshot: AucConf {
            bid_token: app.get_addr("s20"),
            auc_period: 100,
            resv_boundary: 500,
            min_bid_inc: 1000u32,
            unlock_threshold: Uint128(5_000),
        },
    };
    assert_eq!(auc_status, exp_auc_status);

    // same user tries to bid below min bid increment -> fails  
    error = extract_error_msg(sim_bid(&mut app, 77, None));
    assert!(error.contains("bid needs to be at least "));

    // same user can bid again, above the min bid inc  
    sim_bid(&mut app, 85, None).unwrap();
    (bid, pos) = may_get_bid_from_addr(&app.deps.storage, &app.get_addr("user2").address).unwrap().unwrap();
    exp_bid.amount = Uint128(85);
    assert_eq!(bid, exp_bid);
    assert_eq!(pos, 1u32);
    // auction status (live and end height) unchanged
    auc_status = auction_info_r(&app.deps.storage).load().unwrap();
    assert_eq!(auc_status, exp_auc_status);
    
    // another user, user1, can bid, even at last block before auction closes
    app.next_block(100);
    sim_bid(&mut app, 95, Some("user1")).unwrap();
    (bid, pos) = get_last_bid(&app.deps.storage).unwrap();
    exp_bid.bidder = app.get_addr("user1").address;
    exp_bid.amount = Uint128(95);
    assert_eq!(bid, exp_bid);
    assert_eq!(pos, 2u32);
    // auction status (live and end height) unchanged
    auc_status = auction_info_r(&app.deps.storage).load().unwrap();
    assert_eq!(auc_status, exp_auc_status);

    // cannot bid after auction closes, even if finalization tx not called yet
    app.next_block(1);
    error = extract_error_msg(sim_bid(&mut app, 95, None));
    assert!(error.contains("auction has closed"));
    
    // cannot retrieve bid until auction is finalized


    // cannot claim pro-rata sale proceeds until auction is finalized 



    // user0 can call finalization tx
    app.change_env("user0", "ft");
    sim_finalize_auction(&mut app).unwrap();

    // user1 now has nft
    let token: s721::token::Token = json_load(
        &ReadonlyPrefixedStorage::new(PREFIX_INFOS, &app.deps.storage), &0u32.to_le_bytes()
    ).unwrap();
    assert_eq!(app.deps.api.human_address(&token.owner).unwrap(), app.get_addr("user1").address);

    // cannot bid after auction finalized
    error = extract_error_msg(sim_bid(&mut app, 1000, None));
    assert!(error.contains("vault no longer active"));

    // user2 can retrieve its bid
    // before retrieving bid
    assert_eq!(Uint128(5_000 - 85), s20_balance(&mut app, "user2"));
    sim_retrieve_bid(&mut app, "user2").unwrap();
    assert_eq!(Uint128(5_000 - 85 + 85), s20_balance(&mut app, "user2"));
    // user2 cannot retrieve bid again
    error = extract_error_msg(sim_retrieve_bid(&mut app, "user2"));
    assert!(error.contains("you have already retrieved bid"));
    // and balance is unchanged
    assert_eq!(Uint128(5_000 - 85 + 85), s20_balance(&mut app, "user2"));

    // user0 cannot retrieve bid because did not bid
    error = extract_error_msg(sim_retrieve_bid(&mut app, "user0"));
    assert!(error.contains("you did not bid"));
    assert_eq!(Uint128(5_000), s20_balance(&mut app, "user0"));

    // user1 cannot retrieve bid 
    assert_eq!(Uint128(5_000 - 95), s20_balance(&mut app, "user1"));
    error = extract_error_msg(sim_retrieve_bid(&mut app, "user1"));
    assert!(error.contains("you won the bid. You should have received the NFT"));
    assert_eq!(Uint128(5_000 - 95), s20_balance(&mut app, "user1"));

    // unstake all ftokens (note user0 already unstaked 40, so has 70-40 = 30 ftokens left, 
    // similar amount to user1)
    app.change_env("user0", "ft");
    msg = ft::msg::HandleMsg::Unstake { amount: Uint128(30) };
    ft::contract::handle(&mut app.deps, app.env.clone(), msg.clone()).unwrap();
    app.change_env("user1", "ft");
    ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();

    // user0 and user1 and claim pro-rata sale proceeds
    sim_claim_proceeds(&mut app, "user0").unwrap();
    sim_claim_proceeds(&mut app, "user1").unwrap();
    // 5000 is sscrt initial balance
    assert_eq!(Uint128(5_000 + 70*95/100), s20_balance(&mut app, "user0"));
    assert_eq!(Uint128(5_000 - 95 + 30*95/100), s20_balance(&mut app, "user1"));
}

#[test]
fn test_auction_config_reflects_in_new_auction() {
}

/// A snapshot of auction config is taken when an auction is initiated. Proposals to change config
/// must not affect the live auction 
#[test]
fn test_auction_config_does_not_change() {
}

/// Can only be called at the right time
#[test]
fn test_finalize_auction() {
}

#[test]
fn test_proposals() {
    let mut app = App::new();
    init_default(&mut app);
    fractionalize_default(&mut app);
    transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 70, 30).unwrap();


}

#[test]
fn test_proposal_votes() {
}


// #[test]
// fn test_bidding_retrievenft_forced() {
//     let mut app = App::new();
//     init_default(&mut app);
//     fractionalize_default(&mut app);
//     sim_bid(&mut app, 2_000, "user2").unwrap();
//     transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 0, 0).unwrap();

//     // check bid_info is correct
//     let mut exp_bid_info = BidInfo {
//         bid_id: 0u32,
//         bidder: app.get_addr("user2").address,
//         amount: Uint128(2_000),
//         status: BidStatus::Active,
//         end_height: 100u64,
//     };
//     let mut bid_info_0 = bids_r(&app.deps.storage).load(&0u32.to_le_bytes()).unwrap();
//     assert_eq!(bid_info_0, exp_bid_info);
//     assert_eq!(Uint128(3_000), s20_balance(&mut app, "user2"));

//     // user2 can make another bid
//     sim_bid(&mut app, 3_000, "user2").unwrap();
//     exp_bid_info.bid_id = 1u32;
//     exp_bid_info.amount = Uint128(3_000);
//     let mut bid_info_1 = bids_r(&app.deps.storage).load(&1u32.to_le_bytes()).unwrap();
//     assert_eq!(bid_info_1, exp_bid_info);
//     // user2 now has zero s20 (sscrt) token balance
//     assert_eq!(Uint128(0), s20_balance(&mut app, "user2"));

//     // user2 cannot retrieve nft
//     let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
//     let error = extract_error_msg(handle_result);
//     assert!(error.contains("Cannot retrieve underlying NFT: bid status is not `WonInVault`"));

//     // forcefully change bid_status of bid_id: 0. user2 cannot `RetrieveNft`
//     let status_vec = vec![BidStatus::WonRetrieved, BidStatus::Active, BidStatus::LostInTreasury, BidStatus::LostRetrieved];
//     for status in status_vec.iter() {
//         bid_info_0.status = status.clone();
//         bids_w(&mut app.deps.storage).save(&0u32.to_le_bytes(), &bid_info_0).unwrap();
//         let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
//         let error = extract_error_msg(handle_result);
//         assert!(error.contains("Cannot retrieve underlying NFT: bid status is not `WonInVault`"));
//     }
    
//     // // forcefully change bid_status of bid_id: 0 to `WonInVault`, and `won_bid_id` = 0:
//     // won_bid_id_w(&mut app.deps.storage).save(&0u32).unwrap(); 
//     // other (user1) cannot retrieve...
//     bid_info_0.status = BidStatus::WonInVault;
//     bids_w(&mut app.deps.storage).save(&0u32.to_le_bytes(), &bid_info_0).unwrap();
//     let handle_result = sim_retrieve_nft(&mut app, 0u32, "user1");
//     let error = extract_error_msg(handle_result);
//     assert!(error.contains("Cannot retrieve underlying NFT: You are not the bidder"));

//     // ...user2 can retrieve nft
//     let handle_result = sim_retrieve_nft(&mut app, 0u32, "user2");
//     assert!(handle_result.is_ok());
//     // user2 now has NFT
//     let token: s721::token::Token = json_load(
//         &ReadonlyPrefixedStorage::new(PREFIX_INFOS, &app.deps.storage), &0u32.to_le_bytes()
//     ).unwrap();
//     assert_eq!(app.deps.api.human_address(&token.owner).unwrap(), app.get_addr("user2").address);
//     // user2 still has zero s20 (sscrt) token balance
//     assert_eq!(Uint128(0), s20_balance(&mut app, "user2"));

//     // forcefully change bid_status of bid_id: 1. user2 cannot `RetrieveBid`
//     let status_vec = vec![BidStatus::WonRetrieved, BidStatus::WonInVault, BidStatus::Active, BidStatus::LostRetrieved];
//     for status in status_vec.iter() {
//         bid_info_1.status = status.clone();
//         bids_w(&mut app.deps.storage).save(&1u32.to_le_bytes(), &bid_info_1).unwrap();
//         let handle_result = sim_retrieve_bid(&mut app, 1u32, "user2");
//         let error = extract_error_msg(handle_result);
//         assert!(error.contains("Cannot retrieve bid tokens: bid status is not `LostInTreasury`"));
//     }
//     // forcefully change bid_status of bid_id: 1 to `LostInTreasury`: 
//     // other (user1) cannot retrieve...
//     bid_info_1.status = BidStatus::LostInTreasury;
//     bids_w(&mut app.deps.storage).save(&1u32.to_le_bytes(), &bid_info_1).unwrap();
//     let handle_result = sim_retrieve_bid(&mut app, 1u32, "user1");
//     let error = extract_error_msg(handle_result);
//     assert!(error.contains("Cannot retrieve bid tokens: You are not the bidder"));
    
//     // ...user2 can retrieve bid
//     let handle_result = sim_retrieve_bid(&mut app, 1u32, "user2");
//     assert!(handle_result.is_ok());
//     // user2 now has 3000 s20 (sscrt) token balance
//     assert_eq!(Uint128(3_000), s20_balance(&mut app, "user2"));

//     // user0 and user1 can claim pro-rata sale proceeds 
//     assert_eq!(Uint128(5_000), s20_balance(&mut app, "user0"));
//     assert_eq!(Uint128(5_000), s20_balance(&mut app, "user1"));
//     sim_claim_proceeds(&mut app, "user0").unwrap();
//     sim_claim_proceeds(&mut app, "user1").unwrap();
//     assert_eq!(Uint128(5_000 + 70*2_000/100), s20_balance(&mut app, "user0"));
//     assert_eq!(Uint128(5_000 + 30*2_000/100), s20_balance(&mut app, "user1"));
// }

// #[test]
// fn test_bid_votes() {
//     let mut app = App::new();
//     init_default(&mut app);
//     fractionalize_default(&mut app);
//     transfer_ftkn_and_stake(&mut app, "user0", "user1", 30, 70, 30).unwrap();
//     sim_bid(&mut app, 2_000, "user2").unwrap();
//     // todo

//     // // unstake ftokens
//     // app.change_env("user0", "ft");
//     // let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(70) };
//     // ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();

//     // app.change_env("user1", "ft");
//     // let msg = ft::msg::HandleMsg::Unstake { amount: Uint128(30) };
//     // ft::contract::handle(&mut app.deps, app.env.clone(), msg).unwrap();

    
// }


