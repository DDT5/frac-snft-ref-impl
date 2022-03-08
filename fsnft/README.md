
Fractionalized Secret NFT ("frac-sNFT") Standard Specifications and Reference Implementation  <!-- omit in toc --> 
==============
***These are the standard specifications (work in progress) and reference contract (to be built) that implements the base standard required for fractionalized NFTs on Secret Network. The reference contract is designed to be used by developers as-is or to build upon for individual applications.***

_This document and repository is work in progress._

## Table of contents <!-- omit in toc --> 
- [Introduction](#introduction)
  - [Abstract](#abstract)
  - [Terms](#terms)
- [Base specification](#base-specification)
  - [Design](#design)
    - [Fractionalization](#fractionalization)
    - [DAO](#dao)
    - [Configuring the underlying NFT](#configuring-the-underlying-nft)
    - [ftoken holders' access to private metadata](#ftoken-holders-access-to-private-metadata)
    - [Royalties](#royalties)
    - [Bidding](#bidding)
    - [Other rules or unusual situations](#other-rules-or-unusual-situations)
  - [Messages](#messages)
    - [Depositing a SNIP721 token into the vault](#depositing-a-snip721-token-into-the-vault)
    - [Configuring the underlying NFT](#configuring-the-underlying-nft-1)
    - [Bids](#bids)
  - [Queries](#queries)
    - [Private metadata](#private-metadata)
    - [Active bids](#active-bids)
- [Additional specifications](#additional-specifications)
- [Design decisions](#design-decisions)
  - [Philosophy](#philosophy)
  - [Fractionalization](#fractionalization-1)
  - [Royalties](#royalties-1)
  - [Private metadata viewability](#private-metadata-viewability)
  - [Bidding](#bidding-1)
    - [The ability to bid](#the-ability-to-bid)
    - [Multiple bids](#multiple-bids)
  - [Default settings](#default-settings)
- [More information](#more-information)


# Introduction

## Abstract
This memo describes the standard specifications for fractionalized NFTs on Secret Network. The base specification section describes the minimum requirements contracts MUST comform to in order to be compliant, and the additional specification section describe functionality contracts MAY choose to adopt. 

This repository contains the reference contract (not yet built) that implements the base specification.

The standards here are not based on an Ethereum ERC or CosmWasm CW precedent, in contrast with most other [Secret Network Improvement Proposal (SNIP)](https://github.com/SecretFoundation/SNIPs) standards. This architecture is loosely based on what's widely used today on other chains (as of early 2022) to fractionalize NFTs, with added privacy features and designed to work in the computationally private environment of Secret Network. 

## Terms
*The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).*

This memo uses terms defined below: 

* **NFT** refers to a [SNIP721](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-721.md)-compliant token
* **ftokens** ("fractional tokens") are [SNIP20](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-20.md)-compliant (fungible) tokens that represent fractional ownership of an underlying NFT
* **Vault** is the NFT inventory of the frac-sNFT contract 
* **Underlying NFT** refers to a SNIP721 token that has been deposited into the vault
* **Collection** refers to one or more deposited NFTs that share similar ftokens. For the base specification, a collection simply refers to a unique underlying NFT (terminology can be used interchangeably in this case) 
* **NFT depositor** is the user which deposited the underlying NFT into the vault
* **Fractionalized state** refers to the state the underlying NFT is while locked in the vault
* **User** refers to any Secret Network address that interacts with the frac-sNFT contract(s), which can be either a smart contract address or an address controlled by a person 
* **Buy out** refers to the event where a user unlocks (from the vault) and receives an underlying NFT
* **Bidder** refers to an address that places a bid to buy out the collection
* **Bid deposit** is the bid amount that remains locked in the contract while a bid remains active  
* **Bid treasury** is the section of the contract inventory that holds tokens of bids that have won (ie: buy out sale proceeds) and not yet claimed by ftoken holders
* **DAO deposit** are the ftokens deposited when a DAO proposal is made
* **Secondary royalty** refers to royalty payments arising from sale of ftokens (as opposed to primary royalty, which arises from the sale of the underlying NFT)
* **Royalty treasury** is the section of the contract inventory that holds ftokens representing the accrued secondary royalty payments
* **Unfractionalize** refers to the process of unlocking the underlying NFT from the vault. After an NFT is unfractionalized, it is no longer economically linked to its ftokens 

# Base specification

## Design
The contract allows an existing owner of a [SNIP721](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-721.md)-compliant token on Secret Network to deposit the token into the frac-sNFT vault in exchange for fractional tokens (ftokens), which represent fractional ownership that can be traded between Secret addresses. At any time while the underlying NFT is in the vault, a user can place a bid to buy out the underlying NFT. If the bid is successful, the bidder pays its bid amount and receives the underlying NFT. ftoken holders can then redeem their pro-rata share of the sale proceeds. 

### Fractionalization
The frac-sNFT contract MUST implement a [SNIP721 receiver interface](https://github.com/baedrik/snip721-reference-impl/blob/master/README.md#receiver).

The vault MUST accept SNIP721 tokens if the following conditions are met:
* The NFT depositor is the current owner of the SNIP721 token
* While the NFT is in the vault, no address other than the frac-sNFT contract is able to send messages to the underlying NFT, with the exception of the minter being able to change the metadata if that is how the underlying NFT is configured

Once the user deposits a token into the vault, the token MUST be kept in the contract's inventory for as long as the NFT remains in a fractionalized state. The underlying NFT MUST NOT be transferrable out of the vault (eg: by an address with transfer permissions), other than through a [bidding](#bidding) process. The underlying SNIP721 token MUST have the following configurations when it is held in the vault.

```json
{
  "permission_type": { // or Access_level ... todo
    "transfer": "None" // exact json schema  todo
    }
}
```

The contract MUST mint ftokens. All minted ftokens SHOULD be initially transferred to the owner of the underlying NFT and received on the same transaction as the NFT deposit transaction.

ftokens MUST be SNIP20-compliant (hence fungible), and MUST be unique for each collection. ftokens MAY be traded freely between multiple Secret addresses. 

The following parameters MUST be set on the deposit transaction, which MAY have default values if the NFT depositor does not provide inputs.

```json
{
  "bid_config": { 
    "min_bid": "u32 denominated in uSCRT",
    "voting_period":"number of blocks in u32",
    "quorum": "u32 in basis points (=1/10_000)",
    todo!()
},
  "ftoken_config": {
    "total_supply": "u32",
    todo!()
  },
  "auth_query_config": {
    "threshold": "u32 in basis points (=1/10_000)",
    todo!()
  },
  "dao_config": {
    "voting_period":"number of blocks in u32",
    "quorum": "u32",
    "deposit": "u32 denominated in ftokens",
    todo!()
  },
  "trade_royalty": "u32 in basis points (=1/10_000)" 
}
```
`bid_config:`
| Name          | Type | Description                                                 | Optional | Value If Omitted |
| ------------- | ---- | ----------------------------------------------------------- | -------- | ---------------- |
| min_bid       | u32  | Min bid size to buy out the NFT denominated in uSCRT        | yes      | 0                |
| voting_period | u32  | Num of blocks between start and end of voting period        | yes      | 100_000          |
| quorum        | u32  | Min proportion of votes required to pass with unit 1/10_000 | yes      | 2500             |

`ftoken_config:`
| Name         | Type | Description | Optional | Value If Omitted |
| ------------ | ---- | ----------- | -------- | ---------------- |
| total_supply | u32  |             | No       |                  |

`auth_query_config:`
| Name      | Type | Description                                                  | Optional | Value If Omitted |
| --------- | ---- | ------------------------------------------------------------ | -------- | ---------------- |
| threshold | u32  | Min proportion of ftokens before viewing keys can be created | yes      | 1_000_000        |

`dao_config:`
| Name          | Type | Description                                                 | Optional | Value If Omitted |
| ------------- | ---- | ----------------------------------------------------------- | -------- | ---------------- |
| deposit       | u32  | ftoken deposit required when making DAO proposals           | no       |                  |
| voting_period | u32  | Num of blocks between start and end of voting period        | yes      | 100_000          |
| quorum        | u32  | Min proportion of votes required to pass with unit 1/10_000 | yes      | 2500             |


The vault MUST be able to hold multiple collections in its inventory, but ftoken holders of one collection MUST NOT be able to query, view, or send messages to other collections in the vault. Inventory approvals MUST NOT be allowed. Inventory-level configuration of the vault MUST have the following settings, and MUST NOT be changeable.

```json
{
  "inventory_approvals": {
    "inventory_approvals": []
  }
}
```

### DAO

ftoken holders MUST be entited to participate in certain decisions related to their collection:
* whether a bid is accepted or rejected (see [bidding](#bidding))
* propose or vote on change of configuration of the 
  * underlying NFT
  * ftokens 
  * authenticated queries to the frac-sNFT contract
  * DAO 

Changes in configuration MUST be decided by ftoken holders via a DAO:
* A existing ftoken holder submits a proposed transaction message to be sent to the underlying NFT 
* The proposal stays in voting period for the period set by the DAO parameters
* ftoken holders can vote on whether to accept or reject the change (default setting MAY allow either "yes" or "no")
* If the change is accepted, any Secret address can perform a transaction at any time to trigger the proposed message to be sent to the underlying NFT

A user MUST deposit ftokens (DAO deposit) as determined by DAO parameters when making a proposal. The deposit MUST remain locked until the voting period is over. The deposit SHOULD be returned to the proposer after the voting period is over, unless a spam prevention mechanism is implemented on the DAO (such as veto votes). 

### Configuring the underlying NFT

The following messages MUST be able to be sent to the underlying NFT by the frac-sNFT contract while the NFT is in a fractionalized state (list TBC):
* reveal
* set_global_approval
* set_whitelist_approval
* make_ownership_private (?)
* todo!()

The following messages MUST NOT be sendable to the underlying NFT:
* Any inventory-wide approvals   
* Any transfer approvals

### ftoken holders' access to private metadata

An ftoken holder can query the frac-sNFT contract to attempt to view the private metadata of the underlying NFT. The threshold requirements before ftoken holders are allowed to view private metadata MUST be set at NFT deposit. The frac-sNFT contract MAY allow NFT depositors to choose any threshold from 0% to 100% ftoken ownership before private metadata is viewable by a particular address. 

A user which owns an amount of ftoken above the configured threshold SHOULD be able to create a viewing key to perform authenticated queries on the frac-sNFT contract. The frac-sNFT contract MUST check that the relevant address still meets the threshold requirement before responding to the query.

Note: if a Secret address is given permission to view private metadata through a whitelist approval, it can query the underlying NFT directly, and its viewership ability follows the usual behavior of the underlying NFT, regardless of whether the user is an ftoken holder. 

### Royalties

frac-sNFT contract SHOULD mirror the underlying NFT royalty setting on secondary trades royalties (ie: trades of ftokens). It is RECOMMENDED that this is enforced by the contract, without taking input from the NFT depositor. 

Secondary trade royalties MUST have these core functionalities implemented:
* Each ftoken trade transfers the pro-rata % of trade value to the royalty treasury, in the form of ftokens
* The ftokens in the royalty treasury can be claimed by the royalty recipient at any time
* If a buy out bid is successful, unclaimed ftokens MUST be converted to the sSCRT(?) or native(?) token and transferred to the royalty recipient addresses. 

See example [here](#royalties-1)

### Bidding

At any point while the NFT remains in a fractionalized state, Secret addresses MUST be allowed to submit a bid to buy out the underlying NFT. While a bid is active, the frac-sNFT contract SHOULD allow further bids to be made. Conditions SHOULD be set on bids to prevent spamming:
* a minimum bid amount

Bids MUST accept sSCRT tokens. The bidder's bid deposit MUST remain locked in the frac-sNFT contract while the bid remains active. A bid MUST either "win" or "lose". Either zero or one bids SHALL win.

ftoken holders MUST be allowed to vote on each of the bids. Whether bids are accepted or rejected MUST be determined by DAO parameters. A deterministic rule MUST be used to choose which one of potentially multiple accepted bids is the "winner". In the default implementation, all bids have an ID which starts from 0u32 and increments by a whole number for each subsequent bid. When the frac-sNFT contract tests its bids, it searches for the active bid with the smallest ID. If the bid is accepted, it wins. All other bids lose. If instead the bid with the smallest ID is rejected, it loses, and the contract performs the same actions on the bid with the next-smallest ID.
 
If a bid loses, the bidder MUST be able to claim back its bid deposit. 

If a bid wins, the NFT is unfractionalized, where the following MUST hold true or happen: 
* the winner's bid deposit moves to the bid treasury 
* the winning bidder is able to receive the underlying NFT
* ftoken holders can claim their pro-rata share of the bid amount from the bid treasury
* ftoken holders cannot send any messages to the underlying NFT
* all secondary royalty payments cease to accrue
* no further bids can be made
* any query to the frac-sNFT contract regarding this collection (including queries of the private metadata) receives a response that the NFT has been unfractionalized

### Other rules or unusual situations
* If the NFT expires while in the vault, ...
* If a minter changes private metadata while NFT is vault ... 
* If two bids happen on the same block, and if both gets enough yes votes, the one with the smaller ID is tested first. The ID would have been determined by the sequence of the tx when the block was proposed 
* If Bid 1 is made, then Bid 2. But Bid 2 reaches the end of its voting period before Bid 1 does, due to a DAO vote (shortening the voting period) passing between the two bids. Bid 1 should still be tested first due to having a smaller ID. Bid 2 will remain "in waiting" until Bid 1 reaches the end of its voting period.


## Messages

### Depositing a SNIP721 token into the vault 

**Request**
```json
{
    "nft_contr_addr": HumanAddr(),
    "token_id": "token_ID of SNIP721 token to be deposited",
    ... todo!()
}
```
**Response**
```json
{
  todo!()
}
```
 ### Configuring the underlying NFT

**Request**
```json
{
  "propose_msg": {
    "reveal": {
      todo!() 
    }
  }
  
}
```
**Response**
```json
{
  todo!()
}
```
### Bids
A bidder sends the following message:

**Request**
```json
{
  "bid": {
    "nft_contr_addr": HumanAddr(),
    "token_id": "token_ID of SNIP721 token",
    "bid_amt": "String denominated in uSCRT, payable in sSCRT" ,
    "bid_prd": "u32 number of blocks that bid remains active",
    todo!()
  }
}
```
**Response**
```json
{
  todo!()
}
```

## Queries
Secret addresses MUST be able to query:
* DAO configuration settings.
* ftoken configuration settings.
* active bids.
* 

### Private metadata
The following query allows a ftoken holder to attempt to view the private metadata of the underlying NFT 

**Request**
```json
{
  "query_priv_metadata": {
    "nft_contr_addr": HumanAddr(),
    "token_id": ""
    todo!()
  }
}
```
**Response**
```json
{
  todo!()
}
```

### Active bids

**Request**
```json
{
  "query_actv_bids": {
    "nft_contr_addr": HumanAddr(),
    "token_id": ""
    todo!()
  }
}
```
**Response**
```json
{
  todo!()
}
```

# Additional specifications

Allow NFT depositor to be non-owner which has rights to transfer the SNIP721 token

Private vs public voting. Ability to veto

The frac-sNFT contract SHOULD allow non-transferrable SNIP-722 tokens to be deposited into the vault. 

Also allow query permits for authenticated queries

Configure expiry for viewing keys and permits, or ability to revoke VKs and permits through DAO

Ability to work with SNIP1155

# Design decisions

## Philosophy 

The base standard aims to ensure it performs functions within scope with minimal complexity in order to minimize the surface area of attack. It is generally easier and less error prone to start with a tightly designed base (few features, extensively tested) and add features than to work with a complex feature-rich base where only a portion of the features are required.   

The standards aim to provide applications with tools and flexibility. As such, it does not attempt to solve issues that are use case-specific, such as game theory or tokenomics, or to dictate the "correct" settings to use.

## Fractionalization

The standard implementation does not guarantee that the deposited token is fully SNIP721 compliant, as a guarantee is not practical against a determined bad actor. Applications can perform additional checks or have systems in place to mitigate such risks. 

## Royalties

While a SNIP721 token is locked in the vault, the royalty recipient no longer receives royalty from primary trades. frac-sNFT should respect the royalty configuration of the underlying NFT by mirroring this royalty setting as secondary trades (ie: trades of its ftokens). Each trade of ftoken should result in royalty accrued to the underlying NFT royalty recipient based on the pro-rata trade value. 

When an NFT is bought out, the royalties are automatically converted and transferred to the royalty recipient in order to avoid the situation where artists do not claim their entitled royalties due to being unaware of where their NFT creations have been fractionalized and subsequently unfractionalized. 

An example of the core mechanism: an underlying NFT has royalty set at 2%. When the NFT is fractionalized, secondary royalty is configured at 2% to match the underlying NFT. 100 ftokens are minted, so each represents 1% ownership. Bob sells 5 ftokens to Alice. The secondary trade royalty = 5 * 2% = 0.1 ftokens, which will be transferred to the royalty treasury (representing 5 / 100 * 2% = 0.001 = 0.1% of implied value of the underlying NFT). Alice would receive the remaining 5 - 0.1 = 4.9 ftokens.

## Private metadata viewability

The author of this standard recognizes that there are several practical use case-specific issues to solve with viewership permissions of private metadata. Consistent with the design philosophy, this standard does not dictate the correct way for applications to address these issues. Therefore, the standard gives the flexibility to choose any threshold from 0% to 100% ftoken ownership before private metadata is viewable by a particular address. Applications implementing this standard are free to decide whether or not this flexibility to extended to the NFT depositor. 

## Bidding

### The ability to bid

At least one of the major implementations of fractionalized NFTs on other chains today does not have a bidding process. With that design, a party wishing to buy out the underlying NFT needs to first accumulate 100% of the fractionalized tokens. We believe that such an implementation is flawed because unlocking the underlying NFT becomes infeasible. It follows that the economic link between NFT and fractionalized tokens is drastically reduced.
* Buying 100% of ftokens is infeasible in most cases, because i) some ftokens may be lost due to being permanently locked into contracts or being held by accounts that are no longer active, ii) the last few ftoken holders can "hold the underlying NFT hostage" and either achieve a higher sell price or subvert the decision of the majority of ftoken holders.
* An economic link exists between the fractionalized tokens and the underlying NFT precisely because of the ability to unfractionalize the NFT and receive pro-rata share of the sale proceeds
* If unfractionalization is infeasible, the fractionalized tokens no longer represents fractional ownership of the underlying NFT from an economic point of view.
* Additionally, if certain fractional owners can achieve greater sale prices (in the "hold hostage" situation described above), the ftokens are arguably no longer fungible.

The bidding process required by the standards written here essentially solves these issues.

### Multiple bids 

As multiple bids are allowed, there can be situations where more than one bid receives enough "yes" votes to be accepted. Therefore, the final outcome needs to be determined by a second condition. The standard implements a first-come-first-served rule as the default. Therefore, the final outcome is determined by two conditions:
* A bid is "accepted" based on vote counts at the end of its voting period (determined by DAO parameters)
* A bid "wins" when it is the first to be accepted at the end of its voting period. The bidder of the winning bid successfully buys out the underlying NFT. 

For example, Bidder 1 submits Bid 1 on block 10000, and Bid 2 is submitted on block 10010. When Bid 1 reaches the end of its voting period, the contract counts all votes and determines if the bid is accepted. If Bid 1 is accepted, Bidder 1 pays its bid amount and receives the underlying NFT. Bid 2 and any later bids are automatically rejected. If instead, Bid 1 had been rejected at the end of its voting period, then the contract performs the same actions on Bid 2 to determine if it wins the bid.

However, applications can decide on different criteria to determine which accepted bid "wins".

## Default settings

Certain configurations have default settings. This should not be interpreted as the standard recommending a certain set of configurations. Rather, this is done to allow applications to provide convenience or case-specific default values to its own users.  


# More information

[SNIP721 reference implementation](https://github.com/baedrik/snip721-reference-impl)
