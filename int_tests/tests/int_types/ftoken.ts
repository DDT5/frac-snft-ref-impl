import * as Snip20 from "secretjs/src/extensions/snip20/types";
// import * as Snip721 from "secretjs/src/extensions/snip721/types";
import Metadata  from "./metadata";
import FtokenConf from "./ftokenconf";
import { HumanAddr, Uint128, Binary, u8, u32, u64, UndrNftInfo } from "./utils";

/////////////////////////////////////////////////////////////////////////////////
// Instantiation message
/////////////////////////////////////////////////////////////////////////////////

interface FtokenContrInit {
    ftkn_idx: u32,
    depositor: HumanAddr,
    fract_hash: string,
    nft_info: UndrNftInfo,
    init_resv_price: Uint128,
    ftkn_conf: FtokenConf,
}

interface InitialBalance {
    address: HumanAddr,
    amount: Uint128,
}

interface InitConfig {
    public_total_supply?: boolean,
    enable_deposit?: boolean,
    enable_redeem?: boolean,
    enable_mint?: boolean,
    enable_burn?: boolean,
}

export interface FtokenInitMsg {
    init_info: FtokenContrInit,
    name: string,
    admin?: HumanAddr,
    symbol: string,
    decimals: u8,
    initial_balances?: InitialBalance[],
    prng_seed: Binary,
    config?: InitConfig,
  }

// todo
export interface FtokenInitResponse { }
//     messages: Vec<CosmosMsg<T>>,
//     log: Vec<LogAttribute>,
// }

/////////////////////////////////////////////////////////////////////////////////
// Execute messages
/////////////////////////////////////////////////////////////////////////////////

interface BatchReceiveNft {
    sender: HumanAddr,
    from: HumanAddr,
    token_ids: string[],
    msg?: Binary,
}

interface Bid {
    amount: Uint128
}

interface Stake {
    amount: Uint128,
}

interface Unstake {
    amount: Uint128,
}

interface VoteProposal {
    prop_id: u32,
    vote: Vote,
}

interface FinalizeAuction { }

interface RetrieveBid { }

interface ClaimProceeds { }

interface Propose {
    proposal: Proposal,
    stake: Uint128,
}

interface FinalizeExecuteProp {
    prop_id: u32,
}

interface RetrievePropStake {
    prop_id: u32,
}

interface VoteReservationPrice {
    resv_price: Uint128,
}

export type FtokenHandleMsg = Snip20.Snip20DecreaseAllowanceOptions
    | Snip20.Snip20IncreaseAllowanceOptions
    | Snip20.Snip20SendOptions
    | Snip20.Snip20SetViewingKeyOptions
    | Snip20.Snip20TransferOptions
    // todo
    | BatchReceiveNft
    | Bid
    | Stake
    | Unstake
    | VoteProposal
    | FinalizeAuction
    | RetrieveBid
    | ClaimProceeds
    | Propose
    | FinalizeExecuteProp
    | RetrievePropStake
    | VoteReservationPrice

export type FtokenHandleResponse = {}; // todo

/////////////////////////////////////////////////////////////////////////////////
// Query mesages
/////////////////////////////////////////////////////////////////////////////////

export type FtokenQueryMsg = Snip20.GetAllowanceRequest
    | Snip20.GetAllowanceRequestWithPermit
    | Snip20.GetBalanceRequest
    | Snip20.GetBalanceRequestWithPermit
    | Snip20.GetTokenParamsRequest
    | Snip20.GetTransactionHistoryRequest
    | Snip20.GetTransactionHistoryRequestWithPermit
    | Snip20.GetTransferHistoryRequest
    | Snip20.GetTransferHistoryRequestWithPermit
    // todo

export type FtokenQueryResponse = Snip20.GetAllowanceResponse
    | Snip20.GetBalanceResponse
    | Snip20.GetTokenParamsResponse
    | Snip20.TransactionHistoryResponse
    | Snip20.TransferHistoryResponse
    // todo

/////////////////////////////////////////////////////////////////////////////////
// Private interfaces
/////////////////////////////////////////////////////////////////////////////////

type Vote = "yes" | "no" | "veto" | "abstain";

interface AtHeight { at_height: u64 };
interface AtTime { at_time: u64 };
interface Never {  };
type Expiration = AtHeight | AtTime | Never;

interface ApproveToken {  }
interface All {  }
interface RevokeToken {  }
interface None {  }
type AccessLevel = ApproveToken | All | RevokeToken | None;

interface SetMetadata {
    public_metadata?: Metadata,
    private_metadata?: Metadata,
}

interface Reveal { }
interface MakeOwnershipPrivate { }
interface SetGlobalApproval {
    view_owner?: AccessLevel,
    view_private_metadata?: AccessLevel,
    expires?: Expiration,
}

interface SetWhitelistedApproval {
    address: HumanAddr,
    view_owner?: AccessLevel,
    view_private_metadata?: AccessLevel,
    expires?: Expiration,
}

type AllowedNftMsg = SetMetadata | Reveal | MakeOwnershipPrivate | SetGlobalApproval | SetWhitelistedApproval

interface MsgToNft {
    msg: AllowedNftMsg,
}

interface ChangeConfig {
    config: FtokenConf,
}

type Proposal = MsgToNft | ChangeConfig;

