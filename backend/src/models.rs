use serde::{Deserialize, Serialize};

// --- Types for kelshi-context.json ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KelshiContext {
    pub cursor: String,
    pub markets: Vec<Market>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Market {
    pub can_close_early: bool,
    pub close_time: String,
    pub created_time: String,
    pub early_close_condition: String,
    pub event_ticker: String,
    pub expected_expiration_time: String,
    pub expiration_time: String,
    pub expiration_value: String,
    #[serde(default)]
    pub floor_strike: Option<i64>,
    #[serde(default)]
    pub cap_strike: Option<i64>,
    pub last_price: i64,
    pub last_price_dollars: String,
    pub latest_expiration_time: String,
    pub liquidity: i64,
    pub liquidity_dollars: String,
    pub market_type: String,
    pub no_ask: i64,
    pub no_ask_dollars: String,
    pub no_bid: i64,
    pub no_bid_dollars: String,
    pub no_sub_title: String,
    pub notional_value: i64,
    pub notional_value_dollars: String,
    pub open_interest: i64,
    pub open_interest_fp: String,
    pub open_time: String,
    pub previous_price: i64,
    pub previous_price_dollars: String,
    pub previous_yes_ask: i64,
    pub previous_yes_ask_dollars: String,
    pub previous_yes_bid: i64,
    pub previous_yes_bid_dollars: String,
    pub price_level_structure: String,
    pub price_ranges: Vec<PriceRange>,
    pub response_price_units: String,
    pub result: String,
    pub rules_primary: String,
    pub rules_secondary: String,
    pub settlement_timer_seconds: i64,
    pub status: String,
    pub strike_type: String,
    pub subtitle: String,
    pub tick_size: i64,
    pub ticker: String,
    pub title: String,
    pub updated_time: String,
    pub volume: i64,
    pub volume_24h: i64,
    pub volume_24h_fp: String,
    pub volume_fp: String,
    pub yes_ask: i64,
    pub yes_ask_dollars: String,
    pub yes_bid: i64,
    pub yes_bid_dollars: String,
    pub yes_sub_title: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PriceRange {
    pub end: String,
    pub start: String,
    pub step: String,
}

// --- Types for kelshi-odds.json ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KelshiOdds {
    pub orderbook: Orderbook,
    pub orderbook_fp: OrderbookFp,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Orderbook {
    pub no: Vec<(i64, i64)>,
    pub no_dollars: Vec<(String, i64)>,
    pub yes: Vec<(i64, i64)>,
    pub yes_dollars: Vec<(String, i64)>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OrderbookFp {
    pub no_dollars: Vec<(String, String)>,
    pub yes_dollars: Vec<(String, String)>,
}

// --- Types for perplexity.json ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerplexityResponse {
    pub created_at: i64,
    pub id: String,
    pub model: String,
    pub object: String,
    pub output: Vec<PerplexityOutput>,
    pub status: String,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PerplexityOutput {
    #[serde(rename = "search_results")]
    SearchResults(SearchResults),
    #[serde(rename = "message")]
    Message(PerplexityMessage),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchResults {
    pub queries: Vec<String>,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchResult {
    pub date: String,
    pub id: i64,
    pub last_updated: String,
    pub snippet: String,
    pub source: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerplexityMessage {
    pub content: Vec<Content>,
    pub id: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Content {
    pub text: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Usage {
    pub cost: Cost,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub tool_calls_details: ToolCallsDetails,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Cost {
    pub currency: String,
    pub input_cost: f64,
    pub output_cost: f64,
    pub tool_calls_cost: f64,
    pub total_cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolCallsDetails {
    pub search_web: SearchWeb,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchWeb {
    pub invocation: i64,
}