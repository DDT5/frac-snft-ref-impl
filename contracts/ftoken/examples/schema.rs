use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use ftoken::{
    msg::{HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg},
    ftoken_mod::msg::{FtokenQuery, FtokenAuthQuery, FtokenQueryAnswer, S721HandleMsg}
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(HandleAnswer), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(QueryAnswer), &out_dir);

    export_schema(&schema_for!(FtokenQuery), &out_dir);
    export_schema(&schema_for!(FtokenAuthQuery), &out_dir);
    export_schema(&schema_for!(FtokenQueryAnswer), &out_dir);
    export_schema(&schema_for!(S721HandleMsg), &out_dir);    
}
