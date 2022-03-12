#!/bin/bash


# ------------------------------------------------------------------------
# declare variables and functions
# ------------------------------------------------------------------------
set -eu
set -o pipefail # If anything in a pipeline fails, the pipe's exit status is a failure
# set -x # Print all commands for debugging

declare -a KEY=(a b c d)

declare -A FROM=(
    [a]='-y --from a'
    [b]='-y --from b'
    [c]='-y --from c'
    [d]='-y --from d'
)

# This means we don't need to configure the cli since it uses the preconfigured cli in the docker.
# We define this as a function rather than as an alias because it has more flexible expansion behavior.
# In particular, it's not possible to dynamically expand aliases, but `tx_of` dynamically executes whatever
# we specify in its arguments.
# function secretcli() {
#    docker exec secretdev /usr/bin/secretd "$@"
# }

# Alternative to above, when interactively using cli
# docker exec -it secretdev /bin/bash
function secretcli() {
    secretd "$@"
};


# Just like `echo`, but prints to stderr
function log() {
    echo "$@" >&2
}

# suppress all output to stdout for the command described in the arguments
function quiet() {
    "$@" >/dev/null
}

# suppress all output to stdout and stderr for the command described in the arguments
function silent() {
    "$@" >/dev/null 2>&1
}

# Pad the string in the first argument to 256 bytes, using spaces
function pad_space() {
    printf '%-256s' "$1"
}

function assert_eq() {
    set -e
    local left="$1"
    local right="$2"
    local message

    if [[ "$left" != "$right" ]]; then
        if [ -z ${3+x} ]; then
            local lineno="${BASH_LINENO[0]}"
            message="assertion failed on line $lineno - both sides differ. left: ${left@Q}, right: ${right@Q}"
        else
            message="$3"
        fi
        log "$message"
        return 1
    fi
    log "assert_eq SUCCESS"
    return 0
}

function assert_ne() {
    set -e
    local left="$1"
    local right="$2"
    local message

    if [[ "$left" == "$right" ]]; then
        if [ -z ${3+x} ]; then
            local lineno="${BASH_LINENO[0]}"
            message="assertion failed on line $lineno - both sides are equal. left: ${left@Q}, right: ${right@Q}"
        else
            message="$3"
        fi

        log "$message"
        return 1
    fi
    log "assert_ne PASSED"
    return 0
}

declare -A ADDRESS=(
    [a]="$(secretcli keys show --address a)"
    [b]="$(secretcli keys show --address b)"
    [c]="$(secretcli keys show --address c)"
    [d]="$(secretcli keys show --address d)"
)

declare -A VK=([a]='' [b]='' [c]='' [d]='')

# Generate a label for a contract with a given code id
# This just adds "contract_" before the code id.
function label_by_id() {
    local id="$1"
    echo "contract_$id"
}

# Keep polling the blockchain until the tx completes.
# The first argument is the tx hash.
# The second argument is a message that will be logged after every failed attempt.
# The tx information will be returned.
function wait_for_tx() {
    local tx_hash="$1"
    local message="$2"

    local result

    log "waiting on tx: $tx_hash"
    # secretcli will only print to stdout when it succeeds
    until result="$(secretcli query tx "$tx_hash" 2>/dev/null)"; do
        log "$message"
        sleep 1
    done

    # log out-of-gas events
    if quiet jq -e '.raw_log | startswith("execute contract failed: Out of gas: ") or startswith("out of gas:")' <<<"$result"; then
        log "$(jq -r '.raw_log' <<<"$result")"
    fi

    echo "$result"
}

# This is a wrapper around `wait_for_tx` that also decrypts the response,
# and returns a nonzero status code if the tx failed
function wait_for_compute_tx() {
    local tx_hash="$1"
    local message="$2"
    local return_value=0
    local result
    local decrypted

    result="$(wait_for_tx "$tx_hash" "$message")"
    # log "$result"
    if quiet jq -e '.logs == null' <<<"$result"; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    # log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# If the tx failed, return a nonzero status code.
# The decrypted error or message will be echoed
function check_tx() {
    local tx_hash="$1"
    local result
    local return_value=0

    result="$(secretcli query tx "$tx_hash")"
    if quiet jq -e '.logs == null' <<<"$result"; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# Extract the tx_hash from the output of the command
function tx_of() {
    "$@" | jq -r '.txhash'
}

# Extract the output_data_as_string from the output of the command
function data_of() {
    "$@" | jq -r '.output_data_as_string'
}

function get_err() {
    jq -r '.output_error[]' <<<"$1"
}

function get_generic_err() {
    jq -r '.output_error.generic_err.msg' <<<"$1"
}

# Send a compute transaction and return the tx hash.
# All arguments to this function are passed directly to `secretcli tx compute execute`.
function compute_execute() {
    tx_of secretcli tx compute execute "$@"
}

# Send a query to the contract.
# All arguments to this function are passed directly to `secretcli query compute query`.
function compute_query() {
    secretcli query compute query "$@"
}

function upload_code() {
    # set -e
    local directory="$1"
    local file_name="$2"
    local tx_hash
    local code_id

    #when using secretcli non-interactively
    tx_hash="$(tx_of secretcli tx compute store "code/$directory/$file_name.wasm.gz" ${FROM[a]} --gas 10000000)"
    code_id="$(
        wait_for_tx "$tx_hash" 'waiting for contract upload' |
            jq -r '.logs[0].events[0].attributes[] | select(.key == "code_id") | .value'
    )"

    log "uploaded contract #$code_id"

    echo "$code_id"
}

function instantiate() {
    # set -e
    local code_id="$1"
    local init_msg="$2"

    log 'sending init message:'
    log "${init_msg@Q}"

    local tx_hash
    tx_hash="$(tx_of secretcli tx compute instantiate "$code_id" "$init_msg" --label "$(label_by_id "$code_id")" ${FROM[a]} --gas 10000000)"
    wait_for_tx "$tx_hash" 'waiting for init to complete'
}

# This function uploads and instantiates a contract, and returns the new contract's address
function create_contract() {
    # set -e
    local dir="$1"
    local file_name="$2"
    local init_msg="$3"

    local code_id
    code_id="$(upload_code "$dir" "$file_name")"

    local init_result
    init_result="$(instantiate "$code_id" "$init_msg")"

    if quiet jq -e '.logs == null' <<<"$init_result"; then
        log "$(secretcli query compute tx "$(jq -r '.txhash' <<<"$init_result")")"
        return 1
    fi

    jq -r '.logs[0].events[0].attributes[] | select(.key == "contract_address") | .value' <<<"$init_result"
}


# ------------------------------------------------------------------------
# additional variables and functions
# ------------------------------------------------------------------------

function handle() {
    local contract_addr="$1" 
    local msg="$2" # json msg
    local from="$3" # eg "a"
    
    tx_hash="$(tx_of secretcli tx compute execute "$contract_addr" "$msg" ${FROM[${from}]})"
    echo "$tx_hash"
}

# handle-wait: handle then wait_for_tx. echoes "(secretcli query compute tx "$tx_hash")"
function handle_w() {
    local contract_addr="$1" 
    local msg="$2" # json msg
    local from="$3" # eg "a"

    local msg_wait
    msg_wait="$(echo "$msg" | jq 'keys[0]')"

    tx_hash="$(compute_execute "$contract_addr" "$msg" ${FROM[${from}]})"
    resp="$(wait_for_compute_tx "$tx_hash" "waiting to $msg_wait")"    
    # log $resp
    echo "$resp"
}

function gas_of() {
    local txh="$1"
    local txt="$2"  # describe what the tx is
    local gas

    gas="$(secretcli q tx "$txh" | jq -r '.gas_used')"
    echo "$txt: $gas"
}

function log_gas() {
    local txh="$1"
    local txt="$2"
    tx_gas="$(gas_of "$txh" "$txt")"
    gas_log="$(echo "${gas_log}" $'\n'"${tx_gas}")"
    # log "$gas_log"
}


# ------------------------------------------------------------------------
# create viewing keys
# ------------------------------------------------------------------------

# Create viewing keys, where there is the "api_key_" prefix
function create_vk() {   
    contract_addr="$1"
    declare -A tx_hash=([a]='' [b]='' [c]='' [d]='')
    declare -A viewing_key_response=([a]='' [b]='' [c]='' [d]='')

    local create_viewing_key_message='{"create_viewing_key":{"entropy":"MyPassword123"}}'
    for key in "${KEY[@]}"; do
        log "creating viewing key for \"$key\""
        tx_hash[$key]="$(compute_execute "$contract_addr" "$create_viewing_key_message" ${FROM[$key]})"
    done
    wait_for_tx "${tx_hash[d]}" "waiting for create_vk"

    for key in "${KEY[@]}"; do
        viewing_key_response[$key]="$(data_of secretcli q compute tx "${tx_hash[$key]}")"
        VK[$key]="$(jq -er '.create_viewing_key.key' <<<"${viewing_key_response[$key]}")"
        log "viewing key for \"$key\" set to ${VK[$key]}"
        if [[ "${VK[$key]}" =~ ^api_key_ ]]; then
            log "viewing key \"$key\" seems valid"
        else
            log 'viewing key is invalid'
            return 1
        fi
    done

    # Check that all viewing keys are different despite using the same entropy
    assert_ne "${VK[a]}" "${VK[b]}"
    assert_ne "${VK[b]}" "${VK[c]}"
    assert_ne "${VK[c]}" "${VK[d]}"
}

# SNIP721's viewing keys do not have the "api_key_" prefix
function create_vk_s721() {
    contract_addr="$1"
    # postfix="$2"
    declare -A tx_hash=([a]='' [b]='' [c]='' [d]='')
    declare -A viewing_key_response=([a]='' [b]='' [c]='' [d]='')

    # Create viewing keys
    local create_viewing_key_message='{"create_viewing_key":{"entropy":"MyPassword123"}}'
    # local viewing_key_response
    for key in "${KEY[@]}"; do
        log "creating viewing key for \"$key\""
        tx_hash[$key]="$(compute_execute "$contract_addr" "$create_viewing_key_message" ${FROM[$key]})"
    done
    wait_for_tx "${tx_hash[d]}" "waiting for create_vk"

    for key in "${KEY[@]}"; do
        viewing_key_response[$key]="$(data_of secretcli q compute tx "${tx_hash[$key]}")"
        VK[$key]="$(sed -e 's/^{"viewing_key":{"key":"//'  -e 's/"}} *$//' <<<"${viewing_key_response[$key]}")"
        log "viewing key for \"$key\" set to ${VK[$key]}"
    done

    # Check that all viewing keys are different despite using the same entropy
    assert_ne "${VK[a]}" "${VK[b]}"
    assert_ne "${VK[b]}" "${VK[c]}"
    assert_ne "${VK[c]}" "${VK[d]}"
    
    # foo=bar; eval "$foo"="something"; assert_eq $bar "something"

    # echo "${VK[@]}"
}

# ------------------------------------------------------------------------
# create query permits
# ------------------------------------------------------------------------

function makePermit() {
    local contract_addr="$1"
    local key="$2"

    local PERMIT='{
        "chain_id": "secret-4",
        "account_number": "0",
        "sequence": "0",
        "msgs": [
            {
                "type": "query_permit",
                "value": {
                    "permit_name": "test",
                    "allowed_tokens": [
                        "'"$contract_addr"'"
                    ],
                    "permissions": ["owner"]
                }
            }
        ],
        "fee": {
            "amount": [
                {
                    "denom": "uscrt",
                    "amount": "0"
                }
            ],
            "gas": "1"
        },
        "memo": ""
    }'
    echo "$PERMIT" > ~/code/local/permits/permit"${key}".json
    secretcli tx sign-doc ~/code/local/permits/permit"${key}".json --from "$key" > ~/code/local/permits/sig"${key}".json
    # cat ~/code/local/permits/sig.json
    permit_params='{
        "allowed_tokens": '$(echo "$PERMIT" | jq -r '.msgs[0].value.allowed_tokens')',
        "permit_name": "'"$(echo "$PERMIT" | jq -r '.msgs[0].value.permit_name')"'",
        "chain_id": "'"$(echo "$PERMIT" | jq -r '.chain_id')"'",
        "permissions": '$(echo "$PERMIT" | jq -r '.msgs[0].value.permissions')'
    }'

    permit_q='{
        "params":'$(echo "$permit_params")',
        "signature": '$(cat ~/code/local/permits/sig"${key}".json)'
    }'
    # remove white space
    permit_q="$(echo $permit_q | sed  's/ *//g')"

    return 0
}


# ------------------------------------------------------------------------
# secretd extras
# ------------------------------------------------------------------------
# `output_log` -> `output_logs`; `secretcli` -> `secretd`

# fund account aliases `c` and `d`
function fundCD() {
    secretcli tx bank send "${ADDRESS[a]}" "${ADDRESS[c]}" 1000000000000uscrt -y
    txh="$(tx_of secretcli tx bank send "${ADDRESS[b]}" "${ADDRESS[d]}" 1000000000000uscrt -y)"
    wait_for_tx "$txh" "waiting"

    secretcli query bank balances "${ADDRESS[c]}"
    secretcli query bank balances "${ADDRESS[d]}"
}

# ########################################################################
# Instantiate contracts
# ########################################################################
# secretcli query compute list-code
# secretcli query compute list-contract-by-code 1

function doInit() {
    # upload ftoken (no instantiation)
    # prng_seed="$(echo "foo bar" | base64)"
    # init_msg='{"name":"myftoken","symbol":"FTKN","decimals":6,"prng_seed":"'"$prng_seed"'","initial_balances":[{"address":"'"${ADDRESS[a]}"'","amount":"1000000"}]}'
    ftkn_code_id="$(upload_code '.' "ftoken")"
    # ftoken=
    ftoken_h="$(secretcli query compute list-code | jq -r '.[] | select(.id=='"$ftkn_code_id"') | .data_hash ')"
    
    # instantiate fract contract
    init_msg='{"uploaded_ftoken":{"code_id":'$ftkn_code_id',"code_hash":"'"$ftoken_h"'"}}'
    fract="$(create_contract '.' "fractionalizer" "$init_msg")"
    fract_h="$(secretcli q compute contract-hash "$fract" | sed 's/^0x//')"

    # instantiate SNIP721 contract
    init_msg='{"name":"myNFT","symbol":"NFT","entropy":"foo bar","config":{"public_token_supply":true,"public_owner":true,"enable_sealed_metadata":true,"unwrapped_metadata_is_private":true,"minter_may_update_metadata":true,"owner_may_update_metadata":true,"enable_burn":true}}'
    snip721="$(create_contract './tests/snip721' "snip721contract" "$init_msg")"
    snip721_h="$(secretcli q compute contract-hash "$snip721" | sed 's/^0x//')"


}

# ########################################################################
# test functions
# ########################################################################

# ------------------------------------------------------------------------
# handles
# ------------------------------------------------------------------------
function doHandles() {
    # mint nft
    handle_w "$snip721" '{"mint_nft":{}}' a

    # reveal
    handle_w "$snip721" '{"reveal":{"token_id":"0"}}' a
    
    # change public and private metadata (note need to `reveal` first before private metadata can be changed)
    PuMetDat='{"token_uri":"public_uri_all_can_see"}'
    PrMetDat='{"token_uri":"here_is_private_uri"}'
    handle_w "$snip721" '{"set_metadata":{"token_id":"0","public_metadata":'"$PuMetDat"',"private_metadata":'"$PrMetDat"'}}' a

    # # straight transfer of NFT from ADDRESS[a] to fract and back
    # handle_w $snip721 '{"transfer_nft":{"recipient":"'"$fract"'","token_id":"0"}}' a
    # handle_w $fract '{"transfer_nft":{"nft_contr_addr":"'"$snip721"'","nft_contr_hash":"'"$snip721_h"'","recipient":"'"${ADDRESS[a]}"'","token_id":"0"}}' a

    # set_whitelisted_approval for fract contract to transfer
    msg='{"set_whitelisted_approval":{
        "address":"'"${fract}"'",
        "token_id":"0",
        "view_owner":"approve_token",
        "view_private_metadata":"approve_token",
        "transfer":"approve_token"
        }}'
    msg="$(echo $msg | sed  's/ *//g')"
    handle_w "$snip721" "$msg" a


    # # register receive with SNIP721 contract to enable `send`
    # handle_w "$fract" '{"register_nft_contr":{"reg_addr":"'"$snip721"'","reg_hash":"'"$snip721_h"'"}}' a

    # # transfer NFT from ADDRESS[a] to fract, called by fract contract after getting permission
    # msg="$(echo "heres_some_message_hello" | base64)"
    # handle_w $fract '{"send_nft":{"nft_contr_addr":"'"$snip721"'","nft_contr_hash":"'"$snip721_h"'","contract":"'"$ftoken"'","token_id":"0","msg":"'"$msg"'"}}' a
    # # echo $resp | jq '.output_logs[0].attributes[] | select(.key=="msg") | .value' | base64 -d

    msg='{"fractionalize":{
            "nft_info":{
                "token_id":"0",
                "nft_contr":{
                    "code_hash":"'"$snip721_h"'",
                    "address":"'"$snip721"'"
                }
            },
            "ftkn_conf":{
                "name":"ftokencoin",
                "symbol":"FTKN",
                "supply":"1000000",
                "decimals":6
            }
        }
    }'
    msg="$(echo $msg | sed  's/ *//g')"
    handle_w "$fract" "$msg" a
    ftoken0="$(secretcli query compute list-contract-by-code 1 | jq -r '.[0].address')"  # <-- note: ensure user is able to query the created ftoken contract address

    # transfer some ftokens to address b
    handle_w "$ftoken0" '{"transfer":{"recipient":"'"${ADDRESS[b]}"'","amount":"300000"}}' a
}

function ftokenHandles() {
    handle_w "$ftoken0" '{"mint":{"recipient":"'"${ADDRESS[a]}"'","amount":"1000000"}}'
}

# ------------------------------------------------------------------------
# queries
# ------------------------------------------------------------------------
function doQueries() {
    # SNIP721 queries
    compute_query "$snip721" '{"contract_info":{}}'
    compute_query "$snip721" '{"contract_config":{}}'
    compute_query "$snip721" '{"minters":{}}'
    compute_query "$snip721" '{"num_tokens":{}}'
    compute_query "$snip721" '{"all_tokens":{}}'
    compute_query "$snip721" '{"owner_of":{"token_id":"0"}}'
    compute_query "$snip721" '{"nft_info":{"token_id":"0"}}'
    compute_query "$snip721" '{"all_nft_info":{"token_id":"0"}}'
    compute_query "$snip721" '{"private_metadata":{"token_id":"0","viewer":{"address":"'"${ADDRESS[a]}"'","viewing_key":"'"${VK_nft[a]}"'"} }}' 
    compute_query "$snip721" '{"nft_dossier":{"token_id":"0"}}' | jq
    compute_query "$snip721" '{"batch_nft_dossier":{"token_ids":["0"]}}'
    compute_query "$snip721" '{"token_approvals":{"token_id":"0","viewing_key":"'"${VK_nft[a]}"'"}}' | jq
    compute_query "$snip721" '{"inventory_approvals":{"address":"'"${ADDRESS[a]}"'","viewing_key":"'"${VK_nft[a]}"'"}}' | jq
    
    # compute_query $snip721 '{"with_permit":{"permit":'$(echo $permit_q)',"query":{"nft_dossier":{"token_id":"0"}}}}' 
    compute_query "$snip721" '{"with_permit":{"permit":'"$permit_q"',"query":{"nft_dossier":{"token_id":"0"}}}}' 
    
    # ftoken (SNIP20) queries
    compute_query "$ftoken0" '{"token_info":{}}'
    compute_query "$ftoken0" '{"token_config":{}}'
    compute_query "$ftoken0" '{"contract_status":{}}'
    compute_query "$ftoken0" '{"exchange_rate":{}}'
    compute_query "$ftoken0" '{"minters":{}}'
    compute_query "$ftoken0" '{"balance":{"address":"'"${ADDRESS[a]}"'","key":"'"${VK_token[a]}"'"}}'
    
    compute_query "$ftoken0" '{"debug_query":{}}' | jq '.debug_q_answer.nftviewingkey'
    
    
}

# ########################################################################
# Execute tests and print gas_log
# ########################################################################
function main() {
    fundCD
    doInit
    create_vk_s721 "$snip721"; declare -g -A VK_nft=([a]="${VK[a]}" [b]="${VK[b]}" [c]="${VK[c]}" [d]="${VK[d]}")
    # makePermit $snip721 ${KEY[0]}
    doHandles
    create_vk "$ftoken0"; declare -g -A VK_token=([a]="${VK[a]}" [b]="${VK[b]}" [c]="${VK[c]}" [d]="${VK[d]}")
    # doQueries

    # print gas_log and success msg
    gas_log="$(echo "Gas used by" $'\n')"
    echo -e "\n$gas_log"
    echo -e "\nALL TESTS COMPLETED SUCCESSFULLY"
    return 0
}

main "$@"