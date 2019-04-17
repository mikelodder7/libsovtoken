extern crate libc;
extern crate sovtoken;
extern crate indyrs as indy;                      // lib-sdk project
extern crate bs58;

#[macro_use] extern crate log;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;

use std::ptr;
use std::ffi::CString;
use std::os::raw::c_char;

use indy::future::Future;

use sovtoken::logic::address;
use sovtoken::logic::parsers::common::TXO;
use sovtoken::{ErrorCode, IndyHandle};
use sovtoken::utils::constants::txn_types::XFER_PUBLIC;
use sovtoken::utils::results::ResultHandler;
use sovtoken::utils::ffi_support::{c_pointer_from_string, c_pointer_from_str};
use sovtoken::utils::test::callbacks;

mod utils;
use utils::wallet::Wallet;
use utils::setup::{SetupConfig, Setup};


// ***** HELPER TEST DATA  *****

const COMMAND_HANDLE:i32 = 10;
static INVALID_OUTPUT_JSON: &'static str = r#"{"totally" : "Not a Number", "bobby" : "DROP ALL TABLES"}"#;
static VALID_OUTPUT_JSON: &'static str = r#"{"outputs":[["AesjahdahudgaiuNotARealAKeyygigfuigraiudgfasfhja",10]]}"#;
const WALLET_HANDLE:i32 = 0;
const CB : Option<extern fn(_command_handle_: i32, err: i32, payment_req_json: *const c_char) -> i32 > = Some(utils::callbacks::empty_callback);

// ***** HELPER METHODS *****
fn build_payment_req(wallet_handle: IndyHandle, did: &str, inputs: &str, outputs: &str, extra: Option<String>) -> Result<String, ErrorCode> {
    let (receiver, command_handle, cb) =  callbacks::cb_ec_string();

    let extra = extra.map(c_pointer_from_string).unwrap_or(std::ptr::null());

    let error_code = sovtoken::api::build_payment_req_handler(
        command_handle,
        wallet_handle,
        c_pointer_from_str(did),
        c_pointer_from_str(inputs),
        c_pointer_from_str(outputs),
        extra,
        cb
    );

    return ResultHandler::one(ErrorCode::from(error_code), receiver);
}

fn parse_payment_response(response: &str) -> Result<String, ErrorCode> {
    let (receiver, command_handle, cb) =  callbacks::cb_ec_string();

    let error_code = sovtoken::api::parse_payment_response_handler(
        command_handle,
        c_pointer_from_str(response),
        cb
    );

    return ResultHandler::one(ErrorCode::from(error_code), receiver);
}

fn generate_payment_addresses(wallet: &Wallet) -> (Vec<String>, Vec<String>) {
    let seeds = vec![
        str::repeat("2", 32),
        str::repeat("3", 32),
        str::repeat("4", 32),
        str::repeat("1", 32),
    ];

    let payment_addresses = utils::payment::address::generate_n_seeded(wallet, seeds);

    payment_addresses
        .iter()
        .enumerate()
        .for_each(|(idx, address)| debug!("payment_address[{:?}] = {:?}", idx, address));

    let addresses = payment_addresses
        .iter()
        .map(|address| address::unqualified_address_from_address(&address).unwrap())
        .collect();

    return (payment_addresses, addresses);
}

fn get_resp_for_payment_req(pool_handle: i32, wallet_handle: i32, did: &str,
                            inputs: &str, outputs: &str) -> Result<String, ErrorCode> {
    let req = build_payment_req(wallet_handle, did, inputs, outputs, None).unwrap();
    let res = indy::ledger::submit_request(pool_handle, &req).wait().unwrap();
    parse_payment_response(&res)
}

// ***** UNIT TESTS ****

// the build_mint_txn_handler requires a callback and this test ensures that we
// receive an error when no callback is provided
#[test]
fn errors_with_no_call_back() {
    let return_error = sovtoken::api::build_payment_req_handler(COMMAND_HANDLE,
                                                                WALLET_HANDLE,
                                                                ptr::null(),
                                                                ptr::null(),
                                                                ptr::null(),
                                                                ptr::null(),
                                                                None);
    assert_eq!(return_error, ErrorCode::CommonInvalidStructure as i32, "Expecting Callback for 'build_payment_req_handler'");
}

// the build payment req handler method requires an inputs_json parameter and this test ensures that
// a error is returned when no config is provided
#[test]
fn errors_with_no_inputs_json() {
    let return_error = sovtoken::api::build_payment_req_handler(COMMAND_HANDLE,
                                                                WALLET_HANDLE,
                                                                ptr::null(),
                                                                ptr::null(),
                                                                ptr::null(),
                                                                ptr::null(),
                                                                CB);
    assert_eq!(return_error, ErrorCode::CommonInvalidStructure as i32, "Expecting inputs_json for 'build_payment_req_handler'");
}

// the build payment req handler method requires an outputs_json parameter and this test ensures that
// a error is returned when no config is provided
#[test]
fn errors_with_no_outputs_json() {
    let input_json :CString = CString::new(INVALID_OUTPUT_JSON).unwrap();
    let input_json_ptr = input_json.as_ptr();
    let return_error = sovtoken::api::build_payment_req_handler(COMMAND_HANDLE,
                                                                WALLET_HANDLE,
                                                                ptr::null(),
                                                                input_json_ptr,
                                                                ptr::null(),
                                                                ptr::null(),
                                                                CB);
    assert_eq!(return_error, ErrorCode::CommonInvalidStructure as i32, "Expecting outputs_json for 'build_payment_req_handler'");
}

// the build payment req handler method requires an submitter_did parameter and this test ensures that
// a error is returned when no config is provided
#[test]
fn errors_with_no_submitter_did_json() {
    let input_json :CString = CString::new(INVALID_OUTPUT_JSON).unwrap();
    let input_json_ptr = input_json.as_ptr();
    let output_json :CString = CString::new(VALID_OUTPUT_JSON).unwrap();
    let output_json_ptr = output_json.as_ptr();

    let return_error = sovtoken::api::build_payment_req_handler(COMMAND_HANDLE,
                                                                WALLET_HANDLE,
                                                                ptr::null(),
                                                                input_json_ptr,
                                                                output_json_ptr,
                                                                ptr::null(),
                                                                CB);
    assert_eq!(return_error, ErrorCode::CommonInvalidStructure as i32, "Expecting outputs_json for 'build_payment_req_handler'");
}

#[test]
fn success_signed_request() {
    sovtoken::api::sovtoken_init();

    let did = String::from("287asdjkh2323kjnbakjs");

    let wallet = Wallet::new();
    debug!("wallet id = {:?}", wallet.handle);

    let (payment_addresses, addresses) = generate_payment_addresses(&wallet);
    let txo_1 = TXO { address: payment_addresses[0].clone(), seq_no: 1 }.to_libindy_string().unwrap();
    let txo_2 = TXO { address: payment_addresses[1].clone(), seq_no: 1 }.to_libindy_string().unwrap();

    let inputs = json!([
            txo_1, txo_2
        ]);

    let outputs = json!([
            {
                "recipient": payment_addresses[2],
                "amount": 10
            },
            {
                "recipient": payment_addresses[3],
                "amount": 22
            }
        ]);

    let expected_operation = json!({
        "type": XFER_PUBLIC,
        "inputs": [
            {"address": addresses[0], "seqNo": 1},
            {"address": addresses[1], "seqNo": 1}
        ],
        "outputs": [
            {"address": addresses[2], "amount": 10},
            {"address": addresses[3], "amount": 22},
        ],
        "signatures": [
            "bnuZUPAq5jgpqvaQBzXKBQ973yCpjL1pkqJjiBtVPybpzzKGnPv3uE3VufBVZtR6hq2y55b8MSJpPFVMqskBy3m",
            "4HpwuknWrSpJCs2qXEMZA1kbAsP9WxJFaoHq1cH7W3yxLg5R2fHV8QPdY5Hz2bgDmGkRitLaPa3HbF65kTxNpNTe"
        ]
    });

    trace!("Calling build_payment_req");

    let request_string = build_payment_req(
        wallet.handle,
        &did,
        &inputs.to_string(),
        &outputs.to_string(),
        None
    ).unwrap();

    let request: serde_json::value::Value = serde_json::from_str(&request_string).unwrap();
    debug!("Received request {:?}", request);

    assert_eq!(&expected_operation, request.get("operation").unwrap());
    let ident = bs58::decode(&addresses[0]).with_check(None).into_vec().unwrap();
    let ident = bs58::encode(ident).into_string();
    assert_eq!(&ident, request.get("identifier").unwrap().as_str().unwrap());
    assert!(request.get("reqId").is_some());
}

#[test]
pub fn build_and_submit_payment_req() {
    let wallet = Wallet::new();
    let setup = Setup::new(&wallet, SetupConfig {
        num_addresses: 2,
        num_trustees: 4,
        num_users: 0,
        mint_tokens: Some(vec![30]),
        fees: None,
    });
    let payment_addresses = &setup.addresses;
    let pool_handle = setup.pool_handle;
    let dids = setup.trustees.dids();


    let utxo = utils::payment::get_utxo::get_first_utxo_txo_for_payment_address(&wallet, pool_handle, dids[0], &payment_addresses[0]);

    let inputs = json!([utxo]).to_string();
    let outputs = json!([
        {
            "recipient": payment_addresses[1],
            "amount": 20
        },
        {
            "recipient": payment_addresses[0],
            "amount": 10
        }
    ]).to_string();
    let res = get_resp_for_payment_req(pool_handle, wallet.handle, dids[0], &inputs, &outputs).unwrap();

    let res_parsed: serde_json::Value = serde_json::from_str(&res).unwrap();
    let utxos = res_parsed.as_array().unwrap();
    assert_eq!(utxos.len(), 2);

    let value = utxos.get(0).unwrap().as_object().unwrap();
    let pa1_rc = value.get("recipient").unwrap().as_str().unwrap();
    if pa1_rc == payment_addresses[0] {
        assert_eq!(value.get("amount").unwrap().as_i64().unwrap(), 10);
    } else if pa1_rc == payment_addresses[1] {
        assert_eq!(value.get("amount").unwrap().as_i64().unwrap(), 20);
    } else {
        assert!(false);
    }

    let value = utxos.get(1).unwrap().as_object().unwrap();
    let pa1_rc = value.get("recipient").unwrap().as_str().unwrap();
    if pa1_rc == payment_addresses[0] {
        assert_eq!(value.get("amount").unwrap().as_i64().unwrap(), 10);
    } else if pa1_rc == payment_addresses[1] {
        assert_eq!(value.get("amount").unwrap().as_i64().unwrap(), 20);
    } else {
        assert!(false);
    }
}

#[test]
pub fn build_and_submit_payment_req_incorrect_funds() {
    let wallet = Wallet::new();
    let setup = Setup::new(&wallet, SetupConfig {
        num_addresses: 2,
        num_trustees: 4,
        num_users: 0,
        mint_tokens: Some(vec![30]),
        fees: None,
    });
    let addresses = &setup.addresses;
    let pool_handle = setup.pool_handle;
    let dids = setup.trustees.dids();


    let utxo = utils::payment::get_utxo::get_first_utxo_txo_for_payment_address(&wallet,
                                                                                pool_handle, dids[0], &addresses[0]);

    let inputs = json!([utxo]).to_string();
    let outputs_1 = json!([
        {
            "recipient": addresses[1],
            "amount": 20
        },
        {
            "recipient": addresses[0],
            "amount": 20
        }
    ]).to_string();
    let res = get_resp_for_payment_req(pool_handle, wallet.handle, dids[0],
                                       &inputs, &outputs_1).unwrap_err();
    assert_eq!(res, ErrorCode::PaymentInsufficientFundsError);

    let outputs_2 = json!([
        {
            "recipient": addresses[1],
            "amount": 1
        },
        {
            "recipient": addresses[0],
            "amount": 1
        }
    ]).to_string();
    let res = get_resp_for_payment_req(pool_handle, wallet.handle, dids[0],
                                       &inputs, &outputs_2).unwrap_err();
    assert_eq!(res, ErrorCode::PaymentExtraFundsError);
}

#[test]
pub fn build_and_submit_payment_req_with_spent_utxo() {
    let wallet = Wallet::new();
    let setup = Setup::new(&wallet, SetupConfig {
        num_addresses: 3,
        num_trustees: 4,
        num_users: 0,
        mint_tokens: Some(vec![30, 10]),
        fees: None,
    });
    let addresses = &setup.addresses;
    let pool_handle = setup.pool_handle;
    let dids = setup.trustees.dids();

    let utxo = utils::payment::get_utxo::get_first_utxo_txo_for_payment_address(&wallet, pool_handle, dids[0], &addresses[0]);
    let utxo_2 = utils::payment::get_utxo::get_first_utxo_txo_for_payment_address(&wallet, pool_handle, dids[0], &addresses[1]);

    let inputs = json!([utxo_2]).to_string();
    let outputs = json!([
        {
            "recipient": addresses[0],
            "amount": 10
        }
    ]).to_string();
    get_resp_for_payment_req(pool_handle, wallet.handle, dids[0], &inputs, &outputs).unwrap();

    //lets try to spend spent utxo while there are enough funds on the unspent one
    let inputs = json!([utxo_2, utxo]).to_string();
    let outputs = json!([{
        "recipient": addresses[2],
        "amount": 20
    }]).to_string();
    let err = get_resp_for_payment_req(pool_handle, wallet.handle, dids[0], &inputs, &outputs).unwrap_err();
    assert_eq!(err, ErrorCode::PaymentSourceDoesNotExistError);

    //utxo should stay unspent!
    let utxos = utils::payment::get_utxo::send_get_utxo_request(&wallet, pool_handle, dids[0], &addresses[0]);
    assert_eq!(utxos.len(), 2);
    let first_old = utxos[0].source == utxo;
    let second_old = utxos[1].source == utxo;
    assert!(first_old || second_old);
}

#[test]
pub fn build_payment_with_invalid_utxo() {
    sovtoken::api::sovtoken_init();
    let wallet = Wallet::new();
    let (did, _) = indy::did::create_and_store_my_did(wallet.handle, &json!({"seed": "000000000000000000000000Trustee1"}).to_string()).wait().unwrap();

    let inputs = json!(["txo:sov:1234"]).to_string();
    let outputs = json!([
        {
            "recipient": "pay:sov:1234",
            "amount": 10
        }
    ]).to_string();

    let err = build_payment_req(wallet.handle, &did, &inputs, &outputs, None).unwrap_err();
    assert_eq!(err, ErrorCode::CommonInvalidStructure);
}

pub fn build_payment_req_for_not_owned_payment_address() {
    let wallet_1 = Wallet::new();
    let wallet_2 = Wallet::new();

    let setup = Setup::new(&wallet_1, SetupConfig {
        num_addresses: 2,
        num_trustees: 4,
        num_users: 0,
        mint_tokens: Some(vec![30]),
        fees: None,
    });
    let addresses = &setup.addresses;
    let pool_handle = setup.pool_handle;
    let dids = setup.trustees.dids();

    let utxo = utils::payment::get_utxo::get_first_utxo_txo_for_payment_address(&wallet_1, pool_handle, dids[0], &addresses[0]);

    let inputs = json!([utxo]).to_string();
    let outputs = json!([
        {
            "recipient": addresses[1],
            "amount": 30
        }
    ]).to_string();

    let err = build_payment_req(wallet_2.handle, dids[0], &inputs, &outputs, None).unwrap_err();
    assert_eq!(err, ErrorCode::WalletItemNotFound);
}