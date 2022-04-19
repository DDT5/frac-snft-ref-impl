Fractionalized Secret NFT ("frac-sNFT") Standard Specifications and Reference Implementation  <!-- omit in toc --> 
==============
***These are the standard specifications and reference contract that implements the base standard required for fractionalized NFTs on Secret Network. The reference contract is designed to be used by developers as-is or to build upon for individual applications. This document and the standard implementation will continue to evolve.***

***Secondary royalties have not been implemented yet.***


Documentation for the key contracts and packages can be found here:
* [fractionalizer contract](https://ddt5.github.io/Doc/fractionalizer)
* [ftoken contract](https://ddt5.github.io/Doc/ftoken)
* [fsnft_utils](https://ddt5.github.io/Doc/fsnft_utils)


## Workspace organization <!-- omit in toc --> 

This workspace consists of the following contracts:
* Fractionalizer: factory contract that fractionalizes NFTs and instantiates new ftoken contracts
* ftoken: contract that mints SNIP20-compliant tokens that represent fractional ownership of the underlying NFT. Also handles the DAO, treasury, buyout auction process, secondary royalties, and interactions with the underlying NFT. From a code design perspective, most of the additional logic beyond the standard SNIP-20 contract is implemented via the `ftoken_mod` module

The workspace also has the following:
* fsnft_utils: a library of structs, enums and functions that are used across multiple contracts
* int_tests: multi-contract unit tests (in the `src` subfolder) and integration tests (in the `tests` subfolder)


```
Key items in workspace

workspace
├── contracts
│   ├── fractionalizer
│   └── ftoken
│       └── ftoken_mod
├── fsnft_utils
└── int_tests
    ├── src
    └── tests
```

## Table of contents <!-- omit in toc --> 
- [Introduction](#introduction)
  - [Abstract](#abstract)
  - [Terms](#terms)
- [Base specification](#base-specification)
  - [Design](#design)
    - [Fractionalization](#fractionalization)
    - [DAO](#dao)
    - [ftoken holders' access to private metadata](#ftoken-holders-access-to-private-metadata)
    - [Royalties (not yet implemented)](#royalties-not-yet-implemented)
    - [Auction](#auction)
- [Additional specifications](#additional-specifications)
- [Design decisions](#design-decisions)
  - [Philosophy](#philosophy)
  - [Modularity](#modularity)
  - [SNIP721 compliance](#snip721-compliance)
  - [Royalties](#royalties)
  - [Privacy considerations](#privacy-considerations)
  - [Buyout auction](#buyout-auction)
  - [Default settings](#default-settings)
- [More information](#more-information)


# Introduction

## Abstract
This memo describes the standard specifications for fractionalized NFTs on Secret Network. The base specification section describes the minimum requirements contracts MUST comform to in order to be compliant, and the additional specification section describes functionality contracts MAY choose to adopt. 

This repository contains the reference contract that implements the base specification.

The standards here are not based on an Ethereum ERC or CosmWasm CW precedent, in contrast with most other [Secret Network Improvement Proposal (SNIP)](https://github.com/SecretFoundation/SNIPs) standards. This architecture is loosely based on what's widely used today on other chains (as of early 2022) to fractionalize NFTs, with added privacy features and designed to work in the computationally private environment of Secret Network. 

## Terms
*The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).*

This memo uses terms defined below: 

* **NFT** refers to a [SNIP721](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-721.md)-compliant token
* **ftokens** ("fractional tokens") are [SNIP20](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-20.md)-compliant (fungible) tokens that represent fractional ownership of an underlying NFT
* **Underlying NFT** refers to a SNIP721 token that has been fractionalized
* **Vault** is the inventory where NFTs are stored while in fractionalized state   
* **Basket** refers to one or more deposited NFTs within the same vault (which would share similar ftokens). For the base specification, a basket only consists of a single underlying NFT 
* **NFT depositor** is the user which deposited the underlying NFT into the vault
* **User** refers to any Secret Network address that interacts with the frac-sNFT contract(s), which can be either a smart contract address or an address controlled by a person 
* **Unfractionalize** refers to the process of unlocking the underlying NFT from the vault. After an NFT is unfractionalized, it is no longer economically linked to its ftokens  
* **Buy out** happens when a user pays a certain amount to buy the whole underlying NFT. Doing this unfractionalizes the SNIP721 token
* **Bidder** refers to an address that places a bid to buy out the collection
* **DAO deposit** are the ftokens deposited when a DAO proposal is made
* **Secondary royalty** refers to royalty payments arising from sale of ftokens (as opposed to primary royalty, which arises from the sale of the underlying NFT)
* **Royalty treasury** is the section of the contract inventory that holds ftokens representing the accrued secondary royalty payments

# Base specification

## Design
The contract allows an existing owner of a [SNIP721](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-721.md)-compliant token on Secret Network to deposit the token into the frac-sNFT vault in exchange for fractional tokens (ftokens), which represent fractional ownership that can be traded between Secret addresses. Once a threshold proportion of fractional owners have voted on a reservation price, the underlying NFT becomes unlocked. Then, a user can place a bid at or above the reservation price to buy out the underlying NFT through an auction process. If the bid is successful, the bidder pays its bid amount and receives the underlying NFT. ftoken holders can then redeem their pro-rata share of the sale proceeds. 

### Fractionalization
<!-- The frac-sNFT contract MUST implement a [SNIP721 receiver interface](https://github.com/baedrik/snip721-reference-impl/blob/master/README.md#receiver). -->

The vault MUST accept SNIP721 tokens if the following conditions are met:
* The frac-sNFT contract has been given permission to transfer the SNIP721 token (note: this is granted by the owner of the NFT)
* While the NFT is in the vault, no address other than the frac-sNFT contract is able to send messages to the underlying NFT, with the exception of the minter being able to change the metadata if that is how the underlying NFT is configured

Once the user deposits a token into the vault, the token MUST be kept in the contract's inventory for as long as the NFT remains fractionalized. The underlying NFT MUST NOT be transferrable out of the vault (eg: by an address with transfer permissions), other than through an [auction](#auction) process. 

The contract MUST mint ftokens. All minted ftokens SHOULD be initially transferred to the NFT depositor of the underlying NFT and received on the same transaction as the NFT deposit transaction.

ftokens MUST be SNIP20-compliant (hence fungible), and MUST be unique for each basket of NFTs. ftokens MAY be traded freely between multiple Secret addresses. 

The following parameters MUST be set on the deposit transaction, which MAY have default values if the NFT depositor does not provide inputs.

_(Note: the detailed description of the functin of each of these is detailed [here](https://ddt5.github.io/Doc/fsnft_utils/struct.FtokenInit.html))_
```json
{
  "name": "<ftoken name>",
  "symbol": "<ftoken symbol>",
  "supply": "<initial ftokens supply>",
  "decimals": "<smallest denomination>",
  "contract_label": "<instance label>",
  "init_resv_price": "<initial reservation price>",
  "ftkn_conf": {
      "min_ftkn_bond_prd": "<bond period>",
      "priv_metadata_view_threshold": "<private metadata is viewability threshold>",
      "auc_conf": {
          "bid_token": {
            "code_hash": "<hash of bid token contract>",
            "addresss": "<HumanAddr of bid token contract>"
          },
          "auc_period": "<auction period in number of blocks>",
          "resv_boundary": "<determines floor and ceiling of reservation votes>",
          "min_bid_inc": "<minimum bid increment>",
          "unlock_threshold": "<threshold for vault to be unlocked>"
      },
      "prop_conf": { 
          "min_stake": "<stake required to make a proposal>",
          "vote_period": "<voting period in number of blocks>", 
          "vote_quorum": "<proportion of votes required for a vote to pass>", 
          "veto_threshold": "<proportion of votes for a veto to be effective>"
      }
  }
}
```

The fractionalizer contract SHOULD be able to fractionalize multiple NFTs, but being ftoken holders of one vault MUST NOT entitle them be to query, view, or send messages to other vaults. 

### DAO

ftoken holders MUST be entited to participate in certain decisions related to their vault:
* Propose or vote on 
  * changing configuration of the auction process
  * changing configuration of the DAO
  * sending a `SetWhilelistApproval` message to the underlying NFT, which allows a certain address to view its private metadata
  * threshold ownership before a fractional owner can view private metadata via authenticated queries

A user MUST deposit ftokens (DAO deposit) as determined by DAO parameters when making a proposal. The deposit MUST remain locked until the voting period is over. The deposit SHOULD be returned to the proposer after the voting period is over, unless the proposal is vetoed. 

The proposal process is RECOMMENDED to be as follows:
* A existing ftoken holder stakes a certain amount of ftokens, and submits a proposal 
* The proposal stays in voting period for the period set by the DAO parameters
* ftoken holders can vote on whether to accept or reject the change (default setting MUST allow "yes", "no", "abstain" and "veto")
* The outcome of the votes is determined as follows, in this order:
  * If `veto` votes meet the veto threshold, the proposal does not pass and the proposer loses its deposit
  * If total votes (including `abstain`) does not the quorum threshold, the proposal does not pass and the proposer can retrieve its deposit
  * If the total `yes` votes exceed total `no` votes, the the proposal is accepted, any Secret address can perform a transaction at any time to trigger proposal (eg: a transaction message to be sent to the underlying NFT, or a configuration parameter change). The proposer can reclaim its deposit


The following messages MUST be able to be sent to the underlying NFT by the frac-sNFT contract while the NFT is in a fractionalized state:
* reveal
* set_global_approval
* set_whitelist_approval
* make_ownership_private
* set_metadata

The following messages MUST NOT be sendable to the underlying NFT:
* Any transfer approvals

ftoken holders MUST be able to stake their ftokens. Once staked, ftoken holders can vote on proposals. The weight of a given user's vote is determined by the amount staked.

### ftoken holders' access to private metadata

An ftoken holder can query the frac-sNFT contract to attempt to view the private metadata of the underlying NFT. The threshold requirements before ftoken holders are allowed to view private metadata MUST be set at NFT deposit. The frac-sNFT contract MAY allow NFT depositors to choose any threshold from 0% to 100% ftoken ownership before private metadata is viewable by a particular address. 

A user which owns an amount of ftoken above the configured threshold SHOULD be able to perform authenticated queries on the frac-sNFT contract (either through viewing keys or query permits). The frac-sNFT contract MUST check that the relevant address still meets the threshold requirement at the time of query before responding.

Note: if a Secret address is given permission to view private metadata through a whitelist approval, it can query the underlying NFT directly, and its viewership ability follows the usual behavior of the underlying NFT, regardless of whether the user is an ftoken holder. 

### Royalties (not yet implemented)

frac-sNFT contract SHOULD mirror the underlying NFT royalty setting on secondary trades royalties (ie: trades of ftokens). It is RECOMMENDED that this is enforced by the contract, without taking input from the NFT depositor. 

Secondary trade royalties MUST have these core functionalities implemented:
* Each ftoken trade transfers the pro-rata % of trade value to the royalty treasury, in the form of ftokens
* The ftokens in the royalty treasury can be claimed by the royalty recipient at any time
* If a buy out bid is successful, unclaimed ftokens MUST be converted to the bid token and transferred to the royalty recipient addresses. 

See example [here](#royalties-1)

### Auction

At any point while the NFT is fractionalized, it MUST have a reservation price. ftoken holders SHOULD be allowed to vote on the reservation price at any point while the vault is active. The reservation price is determined by the weighted average price of all active (ie: staked) ftoken votes. A price vote MUST be within a ceiling and floor (a configuration that sets the % above and below the current reservation price). Once a threshold proportion of votes is met, the vault becomes unlocked. 

Then, a bidder MAY submit a bid at or above the reservation price. When this happens, an auction process begins. Before the auction closes, further bids SHOULD be allowed, but bids SHALL NOT be withdrawn. An existing bidder SHOULD be allowed to increase its bid. A minimum increment for each subsequent bid SHOULD be set as part of the auction configuration.

Bids MUST accept SNIP20 compliant tokens. The bidder's bid amount MUST remain locked in the frac-sNFT contract while the bid remains active. A bid MUST either "win" or "lose". Only one bid SHALL win.

When an auction is live:
* any configuration changes made through the DAO does not affect the current auction
* (ftoken holders cannot send any messages to the underlying NFT)
 
If a bid loses, the bidder MUST be able to claim back its bid deposit. 

If a bid wins, the NFT is unfractionalized, where the following MUST hold true or happen: 
* the winner must receive the underlying NFT
* ftoken holders can claim their pro-rata share of the winner's bid amount
* all secondary royalty payments cease to accrue
* no further bids can be made
* any query to the frac-sNFT contract regarding this collection (including queries of the private metadata) receives a response that the NFT has been unfractionalized

# Additional specifications

Ability to fractionalize SNIP1155 tokens

Allow NFT depositor to be non-owner which has rights to transfer the SNIP721 token

Private vs public voting. 

The frac-sNFT contract SHOULD allow non-transferrable SNIP-722 tokens to be deposited into the vault. 

Ability to set expiry for viewing keys and permits, or ability to revoke VKs and permits of the underlying NFT through DAO

# Design decisions

## Philosophy 

The base standard aims to ensure it performs functions within scope with minimal complexity in order to minimize the surface area of attack. It is generally easier and less error prone to start with a tightly designed base (few features, extensively tested) and add features than to work with a complex feature-rich base where only a portion of the features are required.   

The standards aim to provide applications with tools and flexibility. As such, it does not attempt to solve issues that are use case-specific, such as game theory or tokenomics, or to dictate the "correct" settings to use.

## Modularity

The ftoken contract is a fork of the [SNIP20 reference contract](https://github.com/scrtlabs/snip20-reference-impl) with significant additional features. The majority of the code for these features are in a separate module `ftoken_mod`. This modular structure is beneficial in many ways: it improves scalability, readibility and flexbility. Also, it allows minimal changes to be made directly on the SNIP20 reference contract, making it easier for developers upgrade to a newer version of the SNIP20 reference contract if needed for their applications. 

## SNIP721 compliance

The standard implementation does not guarantee that the deposited token is fully SNIP721 compliant, as a guarantee is not practical against a determined bad actor. Applications can perform additional checks or have systems in place to mitigate such risks. 

## Royalties
*(Not implemented yet)*

While a SNIP721 token is locked in the vault, the royalty recipient no longer receives royalty from primary trades. frac-sNFT should respect the royalty configuration of the underlying NFT by mirroring this royalty setting as secondary trades (ie: trades of its ftokens). Each trade of ftoken should result in royalty accrued to the underlying NFT royalty recipient based on the pro-rata trade value. 

When an NFT is bought out, the royalties are automatically converted and transferred to the royalty recipient in order to avoid the situation where artists do not claim their entitled royalties due to being unaware of where their NFT creations have been fractionalized and subsequently unfractionalized. 

An example of the core mechanism: an underlying NFT has royalty set at 2%. When the NFT is fractionalized, secondary royalty is configured at 2% to match the underlying NFT. 100 ftokens are minted, so each represents 1% ownership. Bob sells 5 ftokens to Alice. The secondary trade royalty = 5 * 2% = 0.1 ftokens, (representing 5 / 100 * 2% = 0.001 = 0.1% of implied value of the underlying NFT) which will be transferred to the royalty treasury. Alice would receive the remaining 5 - 0.1 = 4.9 ftokens.

And alternative (and more common) approach is for new ftokens to be minted, hence royalty comes in the form of inflation, rather than deduction of tokens when transferred.

## Privacy considerations

There is an inevitable privacy trade off with fractionalizing. Private information cannot be queried directly, but there is an increase surface area of attack (for example using blockchain analysis or side chains) as a fractionalized NFT operates in a more complex environment with many more stakeholders involved. Consistent with the design philosophy, this standard does not dictate the correct way for applications to address these issues. Instead, the standard aims to give flexibility to applications choose where they are positioned along the privacy-convenience tradeoff spectrum, for key settings. For example, it is possible to configure ftoken ownership percentage anywhere from 0% to 100% before private metadata is viewable by a particular fractional owner. Applications implementing this standard are free to decide whether or not this flexibility to extended to their users. 

Note: currently, it is quite straightforward to identify bidding addresses through blockchain analysis. Concealing this information would significantly reduce user experience of both the bidders and the fractional owners. With CosmWasm 1.0, it may be possible to increase the privacy for bidders.

## Buyout auction

An economic link exists between the fractionalized tokens and underlying NFT because a buyer is able to buy out the underlying NFT, and fractionalized token holders receive pro-rata share of the sale proceeds in the process. It is important that there is relatively low friction in this process to maintain a strong economic link. 
* Without an auction process, a party wishing to buy out the underlying NFT needs to accumulate 100% of the fractionalized tokens, which is infeasible in many situations because i) some ftokens may be lost due to being permanently locked into contracts, being held by accounts that are no longer active, or in the form of "dust" during trades, ii) the last few ftoken holders can "hold the underlying NFT hostage" and either achieve a higher sell price or subvert the decision of the majority of ftoken holders. An auction process avoids these problems.
* An auction process ensures that all fractional owners receive the same sale proceeds, pro-rata to their ownership percentage, which is an important feature for the ftokens to remain fungible. It prevents the "hold hostage" situation described above, where some ftoken holders achieve greater sale prices.


## Default settings

Certain configurations in this standard implementation may have default settings. This should not be interpreted as the standard recommending a certain set of configurations. Rather, this is done to allow applications to provide convenience or case-specific default values to its own users.  


# More information

[SNIP721 reference implementation](https://github.com/baedrik/snip721-reference-impl)
