import { SecretNetworkClient, Tx, Coin, toBase64 } from "secretjs";
import fs from "fs";
import assert from "assert";
import { getScrtBalance, initClient, } from "./helpers";
import { 
  Account, UploadedCode, jsContractInfo, jsEnv, 
  InitMsg, InitResponse, HandleMsg, HandleResponse, QueryMsg, QueryResponse, 
} from "./int_types";

/////////////////////////////////////////////////////////////////////////////////
// Global variables
/////////////////////////////////////////////////////////////////////////////////

const gasLimit = 200000;

/////////////////////////////////////////////////////////////////////////////////
// Upload contract code and instantiate messages
/////////////////////////////////////////////////////////////////////////////////

const uploadContractCode = async (
  client: SecretNetworkClient,
  contractPath: string,
): Promise<UploadedCode> => {
  
  const wasmCode = fs.readFileSync(contractPath);
  console.log("Uploading contract");

  const uploadReceipt = await client.tx.compute.storeCode(
    {
      wasmByteCode: wasmCode,
      sender: client.address,
      source: "",
      builder: "",
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 5000000,
    }
  );

  if (uploadReceipt.code !== 0) {
    console.log(
      `Failed to get code id: ${JSON.stringify(uploadReceipt.rawLog)}`
    );
    throw new Error(`Failed to upload contract`);
  }

  const codeIdKv = uploadReceipt.jsonLog![0].events[0].attributes.find(
    (a: any) => {
      return a.key === "code_id";
    }
  );

  const codeId = Number(codeIdKv!.value);
  console.log("Contract codeId: ", codeId);

  const contractCodeHash = await client.query.compute.codeHash(codeId);
  console.log(`Contract hash: ${contractCodeHash}`);

  const uploadedCode = {
    codeId,
    hash: contractCodeHash
  }

  return uploadedCode 
}

async function execInstantiate(
  client: SecretNetworkClient,
  contractCode: UploadedCode,
  initMsg: InitMsg,
): Promise<jsContractInfo> {

  const contract = await client.tx.compute.instantiateContract(
    {
      sender: client.address,
      codeId: contractCode.codeId,
      initMsg: initMsg, 
      codeHash: contractCode.hash,
      label: "Contract " + Math.ceil(Math.random() * 10000) + client.address.slice(6),
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit: 1000000,
    }
  );

  if (contract.code !== 0) {
    throw new Error(
      `Failed to instantiate the contract with the following error ${contract.rawLog}`
    );
  }

  const contractAddress = contract.arrayLog!.find(
    (log) => log.type === "message" && log.key === "contract_address"
  )!.value;

  console.log(`Contract address: ${contractAddress}`);

  // const contractInfo: [string, string] = [contractCodeHash, contractAddress];
  const contractInfo: jsContractInfo = {
    hash: contractCode.hash,
    address: contractAddress,
  }
  return contractInfo;
};

async function initFrac(
  sender: Account,
  fracCode: UploadedCode,
  snip20Code: UploadedCode,
): Promise<jsContractInfo> {
  const initMsg: InitMsg = {
    uploaded_ftoken: {
      code_id: snip20Code.codeId,
      code_hash: snip20Code.hash,
    }
  }
  const { secretjs } = sender;
  const contractInfo = await execInstantiate(secretjs, fracCode, initMsg);
  
  return contractInfo;
}

/////////////////////////////////////////////////////////////////////////////////
// Handle Messages
/////////////////////////////////////////////////////////////////////////////////

async function execHandle(
  sender: Account,
  contract: jsContractInfo,
  msg: HandleMsg,
  handleDescription?: string,
  sendAmount?: number,
): Promise<Tx> {
  let sentFunds: Coin[] = [];
  if (typeof sendAmount === 'number') {
    sentFunds = [{
      denom: "uscrt",
      amount: sendAmount.toString()
    }]
  }

  const { secretjs } = sender;
  const tx = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contractAddress: contract.address,
      codeHash: contract.hash,
      msg,
      sentFunds,
    },
    {
      broadcastCheckIntervalMs: 100,
      gasLimit,
    }
  );

  if (handleDescription === undefined) { handleDescription = "handle"}
  console.log(`${handleDescription} used ${tx.gasUsed} gas`);
  return tx
}

async function execFractionalize(
  sender: Account,
  contract: jsContractInfo,
) {

}

/////////////////////////////////////////////////////////////////////////////////
// Query Messages
/////////////////////////////////////////////////////////////////////////////////

async function execQuery(
  sender: Account,
  contract: jsContractInfo,
  msg: QueryMsg,
): Promise<QueryResponse> {
  const { secretjs } = sender;

  const response: QueryResponse = (await secretjs.query.compute.queryContract({
    contractAddress: contract.address,
    codeHash: contract.hash,
    query: msg,
  }));

  if (JSON.stringify(response).includes('parse_err"')) {
    throw new Error(`Query parse_err: ${JSON.stringify(response)}`);
  }
  
  return response;
}

async function queryGetCount(
  sender: Account,
  contract: jsContractInfo,
) {

}

/////////////////////////////////////////////////////////////////////////////////
// Helper functions
/////////////////////////////////////////////////////////////////////////////////

/**
 * Initialization procedure: Initialize client, fund new accounts, and upload/instantiate contracts 
 * @returns Promise<jsEnv>
 */
async function initDefault(): Promise<jsEnv> {
  const accounts = await initClient();
  const { secretjs } = accounts[0];

  const fracUploadedCode = await uploadContractCode(
    secretjs,
    "fractionalizer.wasm.gz",
  );

  const ftokenUploadedCode = await uploadContractCode(
    secretjs,
    "ftoken.wasm.gz",
  );
  
  const contract = await initFrac(
    accounts[0],
    fracUploadedCode,
    ftokenUploadedCode
  );

  const env: jsEnv = {
    accounts,
    uploadedCodes: [
      fracUploadedCode,
      ftokenUploadedCode
    ],
    contracts: [
      contract
    ],
  }; 

  return env;
}

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

async function testSanity(
  env: jsEnv,
) {
  const user0 = env.accounts[0];
  const frac = env.contracts[0];
  const ftoken = env.contracts[1];

  let p0BalStart = parseInt(await getScrtBalance(user0));

  assert(1===1, "no way...")
}

/////////////////////////////////////////////////////////////////////////////////
// Main
/////////////////////////////////////////////////////////////////////////////////

async function runTest(
  tester: (
    env: jsEnv,
  ) => void,
  env: jsEnv
) {
  console.log(`[TESTING...]: ${tester.name}`);
  await tester(env);
  console.log(`[SUCCESS] ${tester.name}`);
}

(async () => {
  let env: jsEnv;

  env = await initDefault();
  await runTest(testSanity, env);

  console.log("All tests COMPLETED SUCCESSFULLY");

})();
