#![doc = include_str!("../README.md")]

mod parsing;

use create_type_spec_derive::CreateTypeSpec;
use parsing::parse_exchange_rates;
use pbc_contract_common::{
    address::Address,
    context::ContractContext,
    off_chain::{HttpRequestData, OffChainContext},
    sorted_vec_map::SortedVecMap,
};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

/// The api endpoint used for fetching exchange rates
const API_ENDPOINT: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";

/// State of the contract
#[state]
pub struct State {
    /// Signals that the offchain should fetch new exchange rates
    update_exchange_rates: bool,
    /// Timestamp for the last time the exchange rates was updated.
    /// It is formatted as an ISO-8601 date
    last_updated: String,
    /// Name of the distributor of the exchange rates
    source: String,
    /// The stored exchange rates.
    ///
    /// The key is the name of the currency and the value is the
    /// exchange rate from EUR.
    ///
    /// Exchange rates are stored as integers where the last 5 digits are the decimals.
    /// For example:
    /// - 1.1607 is stored as 116070
    /// - 7.4691 is stored as 746910
    /// - 176.45 is stored as 17645000
    exchange_rates: SortedVecMap<String, u64>,
    /// The execution engines connected to the contract
    execution_engine_node: Address,
}

/// List of exchange rates, fetched from a server
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct ExchangeRates {
    /// Timestamp for the last time the exchange rates was updated
    pub timestamp: String,
    /// The source of the exchange rates
    pub source: String,
    /// The exchange rates
    pub exchange_rates: Vec<ExchangeRate>,
}

/// Exchange rate for a currency
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug, PartialEq)]
pub struct ExchangeRate {
    /// The currency of the exchange rate
    pub currency: String,
    /// The rate of the currency compared to EUR
    pub rate: u64,
}

/// Initialize contract with given execution engine nodes
#[init]
pub fn initialize(_ctx: ContractContext, execution_engine_node: Address) -> State {
    State {
        execution_engine_node,
        update_exchange_rates: false,
        exchange_rates: SortedVecMap::new(),
        source: "NOONE".to_string(),
        last_updated: "NEVER".to_string(),
    }
}

/// Signal to off chain that new exchange rates should be fetched
#[action(shortname = 0x01)]
pub fn refresh_exchange_rates(_ctx: ContractContext, mut state: State) -> State {
    state.update_exchange_rates = true;
    state
}

/// Add exchange rates to the state.
///
/// This action is used by the execution engine when it has fetched exchange rates. Only the
/// execution engine can call this action.
#[action(shortname = 0x02)]
pub fn update_exchange_rates(
    ctx: ContractContext,
    mut state: State,
    exchange_rates: ExchangeRates,
) -> State {
    assert!(
        state.execution_engine_node == ctx.sender,
        "Only the assigned engine, {:?}, may update exchange rates",
        state.execution_engine_node
    );

    state.update_exchange_rates = false;

    state.last_updated = exchange_rates.timestamp;
    state.source = exchange_rates.source;
    for exchange_rate in exchange_rates.exchange_rates {
        state
            .exchange_rates
            .insert(exchange_rate.currency, exchange_rate.rate);
    }

    state
}

/// When the "update_exchange_rates" signal is enabled, the connected execution engine fetches new
/// exchange rates and adds them to the state.
#[off_chain_on_state_change]
fn on_state_change(mut ctx: OffChainContext, state: State) {
    if ctx.execution_engine_address != state.execution_engine_node {
        return;
    }

    if !state.update_exchange_rates {
        return;
    }

    let request = HttpRequestData {
        method: "GET".to_string(),
        uri: API_ENDPOINT.to_string(),
        headers: vec![],
        body: vec![],
    };

    let response = ctx.send_http_request(request);

    assert!(
        response.status_code == 200,
        "Invalid status code from server: {}",
        response.status_code
    );

    let body =
        std::str::from_utf8(&response.body).expect("Response from server should be valid utf-8");

    let exchange_rates = parse_exchange_rates(body);

    assert!(
        !exchange_rates.exchange_rates.is_empty(),
        "Could not find any exchange rates from: {body}"
    );

    let rpc = update_exchange_rates::rpc(exchange_rates);
    ctx.send_transaction_to_contract(rpc, 40_000);
}
