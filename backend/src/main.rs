pub mod models;

use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::types::ContentBlock;
use aws_sdk_bedrockruntime::types::ConverseOutput;
use dotenv::dotenv;
use http::Method;
use lambda_http::{service_fn, Body, Error, Request, RequestPayloadExt, Response};
use models::{KelshiContext, PolymarketEvent};
use reqwest;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::SystemTime;
use tokio::sync::OnceCell;
use tracing::{debug, error, info, instrument, warn};

#[derive(Deserialize, Default)]
struct AppRequest {
    #[serde(default)]
    market_url: String,
}

#[derive(Serialize)]
struct AppResponse {
    market_type: String,
    selected_outcome: String,
    prediction: String,
    recommended_buy: String,
    confidence: f32,
    key_factors: Vec<String>,
    risks: Vec<String>,
    time_sensitivity: String,
    alternative_outcomes_considered: Vec<String>,
    timestamp: i64,
    model: String,
    data_vs_market_divergence: String,
    market_sentiment_note: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BedrockPrediction {
    #[serde(default)]
    market_type: String,
    #[serde(default)]
    selected_outcome: String,
    prediction: String,
    #[serde(default)]
    recommended_buy: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    data_vs_market_divergence: String,
    #[serde(default)]
    key_factors: Vec<String>,
    #[serde(default)]
    market_sentiment_note: String,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(default = "default_time_sensitivity")]
    time_sensitivity: String,
    #[serde(default)]
    alternative_outcomes_considered: Vec<String>,
}

fn default_confidence() -> f32 { 0.5 }
fn default_time_sensitivity() -> String { "Not specified".to_string() }

static BEDROCK_CLIENT: OnceCell<BedrockClient> = OnceCell::const_new();

/// Initialize AWS Bedrock client
#[instrument]
async fn get_bedrock_client() -> &'static BedrockClient {
    BEDROCK_CLIENT.get_or_init(|| async {
        info!("Initializing AWS Bedrock client for the first time (cold start)");
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        BedrockClient::new(&config)
    }).await
}

#[instrument(skip(url))]
async fn call_external_api(url: &str) -> Result<(String, String, String, String), anyhow::Error> {
    info!(market_url = %url, "Calling external API");
    
    let client = reqwest::Client::new();
    
    // Check if it's a Polymarket URL
    if url.contains("polymarket.com/event/") {
        info!("Detected Polymarket URL");
        
        // Extract slug from polymarket.com/event/{slug}
        let re = Regex::new(r"event/([^/]+)")?;
        let slug = re
            .captures(url)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract slug from Polymarket URL: {}", url))?;
        
        info!(slug = %slug, "Extracted slug from Polymarket URL");
        
        let api_url = format!("https://gamma-api.polymarket.com/events/slug/{}", slug);
        debug!(api_url = %api_url, "Constructed Polymarket API URL");
        
        let response = client.get(&api_url).send().await?;
        let status = response.status();
        info!(status = %status, "Received response from Polymarket API");
        
        if !status.is_success() {
            error!(status = %status, "Polymarket API request failed");
            return Err(anyhow::anyhow!("Polymarket API request failed with status: {}", status));
        }
        
        let response_text = response.text().await?;
        debug!(response_len = response_text.len(), "Received Polymarket API response body");
        
        let polymarket_event: PolymarketEvent = serde_json::from_str(&response_text)?;
        info!(market_count = polymarket_event.markets.len(), title = %polymarket_event.title, "Parsed Polymarket event");
        
        let market_type = polymarket_event.title.clone();
        let market_data_as_string = serde_json::to_string(&polymarket_event.markets)?;
        let odds_data = String::new();
        let volume_data = market_data_as_string;
        let news_context = String::new();
        
        info!(market_type = %market_type, "Extracted Polymarket data successfully");
        
        return Ok((market_type, odds_data, volume_data, news_context));
    }
    
    // Default to Kalshi API
    info!("Detected Kalshi URL");
    
    // Regex to capture the market ticker from the URL and convert it to uppercase.
    let re = Regex::new(r"markets/([^/]+)")?;
    let series_ticker = re
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_uppercase())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract market ticker from URL: {}", url))?;
    
    info!(series_ticker = %series_ticker, "Extracted series ticker from URL");

    let api_url = format!("https://api.elections.kalshi.com/trade-api/v2/markets?series_ticker={}&status=open", series_ticker);
    
    debug!(api_url = %api_url, "Constructed Kalshi API URL");

    let response = client.get(&api_url).send().await?;
    
    let status = response.status();
    info!(status = %status, "Received response from Kalshi API");

    if !status.is_success() {
        error!(status = %status, "Kalshi API request failed");
        return Err(anyhow::anyhow!("Kalshi API request failed with status: {}", status));
    }

    let response_text = response.text().await?;
    debug!(response_len = response_text.len(), "Received Kalshi API response body");
    
    let kalshi_context: KelshiContext = serde_json::from_str(&response_text)?;
    info!(market_count = kalshi_context.markets.len(), "Parsed Kalshi context");

    let market_type = kalshi_context.markets.first().map(|m| m.market_type.clone()).unwrap_or_default();
    let market_data_as_string = serde_json::to_string(&kalshi_context.markets)?;
    // right now - we're not differentiating between odds and volume data, but this can be easily extended in the future if needed by parsing the market data more granularly
    // also we're not getting any news context for now, but this can be added in the future by integrating with a news API and fetching relevant news based on the market's event ticker or other metadata
    let odds_data = String::new();
    let volume_data = market_data_as_string;
    let news_context = String::new();
    
    info!(market_type = %market_type, "Extracted market data successfully");

    Ok((market_type, odds_data, volume_data, news_context))
}

/// Load system prompt from file
// fn load_system_prompt() -> String {
//     std::fs::read_to_string("example-prompt.txt")
//         .unwrap_or_else(|_| {
//             warn!("Failed to read example-prompt.txt, using fallback prompt");
//             "Role: Expert prediction market analyst
//             Task: Analyze the complete market question and ALL possible outcomes: {}
//             Data: [ODDS_DATA] {}, [VOLUME_DATA] {}, [NEWS_CONTEXT] {}
//             MAKE SURE YOU OUTPUT THE PREDICTION IN JSON FORMAT.".to_string()
//         })
// }

/// Sanitize and extract relevant data using a lightweight model before prediction
#[instrument(skip(client, odds_data, volume_data, news_context))]
async fn invoke_bedrock_sanitisation_model(
    client: &BedrockClient,
    market_type: &str,
    odds_data: &str,
    volume_data: &str,
    news_context: &str,
) -> Result<(String, String, String, String), anyhow::Error> {
    let model_id = "au.anthropic.claude-haiku-4-5-20251001-v1:0";
    info!(model_id = %model_id, market_type = %market_type, "Invoking Bedrock sanitisation model");

    let system_prompt = format!(
        "You are a data preprocessing assistant for a prediction market analysis system. Your task is to sanitize and extract the most relevant information from raw market data.

**Input Data:**
- Market Type: {}
- Odds Data: {}
- Volume Data: {}
- News Context: {}

**Instructions:**
1. Analyze the raw market data and extract only the most relevant information for prediction analysis
2. Clean up any redundant, noisy, or irrelevant data
3. Structure the data to highlight key market indicators, recent price movements, volume trends, and relevant news
4. Summarize lengthy content into concise, actionable insights
5. Standardize format for downstream processing

**CRITICAL OUTPUT INSTRUCTION:**
You must output ONLY a raw JSON object. Do NOT wrap the JSON in markdown code blocks (no ```json or ```). Do NOT add any explanatory text before or after the JSON. Your entire response must be valid, parseable JSON starting with '{{' and ending with '}}'.

**JSON Output Format:**
{{\n    \"sanitizedMarketType\": \"Clean, concise market description\",\n    \"relevantOdds\": \"Key odds information only\",\n    \"relevantVolume\": \"Volume trends and significant metrics\",\n    \"relevantNews\": \"Summarized news context or empty if none\"\n}}",
        market_type, odds_data, volume_data, news_context
    );

    let user_message = format!(
        "Sanitize and extract relevant data for the {} market.",
        market_type
    );

    let content = ContentBlock::Text(user_message);

    let message = aws_sdk_bedrockruntime::types::Message::builder()
        .role(aws_sdk_bedrockruntime::types::ConversationRole::User)
        .content(content)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build message: {}", e))?;

    info!("Sending sanitisation request to Bedrock");

    let response = client
        .converse()
        .model_id(model_id)
        .system(aws_sdk_bedrockruntime::types::SystemContentBlock::Text(system_prompt))
        .messages(message)
        .send()
        .await;

    match response {
        Ok(output) => {
            info!("Received successful sanitisation response from Bedrock");
            if let Some(ConverseOutput::Message(message)) = output.output {
                if let Some(ContentBlock::Text(text)) = message.content().first() {
                    debug!(response_text_len = text.len(), "Bedrock sanitisation response received");

                    // The model sometimes wraps the JSON in markdown or adds extra text.
                    // We'll find the first '{' and the last '}' to extract the JSON object.
                    let json_start = text.find('{');
                    let json_end = text.rfind('}');

                    let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
                        &text[start..=end]
                    } else {
                        text.as_str()
                    };

                    debug!(json_str = %json_str, "Extracted JSON string from sanitisation response");

                    // Try to parse as JSON to extract sanitized fields
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                        let sanitized_market_type = parsed
                            .get("sanitizedMarketType")
                            .and_then(|v| v.as_str())
                            .unwrap_or(market_type)
                            .to_string();
                        let relevant_odds = parsed
                            .get("relevantOdds")
                            .and_then(|v| v.as_str())
                            .unwrap_or(odds_data)
                            .to_string();
                        let relevant_volume = parsed
                            .get("relevantVolume")
                            .and_then(|v| v.as_str())
                            .unwrap_or(volume_data)
                            .to_string();
                        let relevant_news = parsed
                            .get("relevantNews")
                            .and_then(|v| v.as_str())
                            .unwrap_or(news_context)
                            .to_string();

                        info!("Successfully parsed sanitized data");
                        Ok((sanitized_market_type, relevant_odds, relevant_volume, relevant_news))
                    } else {
                        warn!("Failed to parse sanitisation response as JSON, returning original data");
                        Ok((
                            market_type.to_string(),
                            odds_data.to_string(),
                            volume_data.to_string(),
                            news_context.to_string(),
                        ))
                    }
                } else {
                    error!("No text content in Bedrock sanitisation response");
                    Err(anyhow::anyhow!("No text content in Bedrock sanitisation response"))
                }
            } else {
                error!("Unexpected response type from Bedrock sanitisation");
                Err(anyhow::anyhow!("Unexpected response type from Bedrock sanitisation"))
            }
        }
        Err(e) => {
            error!(error = %e, "Bedrock sanitisation API error");
            Err(anyhow::anyhow!("Failed to invoke Bedrock sanitisation model: {}", e))
        }
    }
}

/// Invoke AWS Bedrock model to generate a prediction
#[instrument(skip(client, odds_data, volume_data, news_context))]
async fn invoke_bedrock_prediction_model(
    client: &BedrockClient,
    market_type: &str,
    odds_data: Option<String>,
    volume_data: Option<String>,
    news_context: Option<String>,
) -> Result<BedrockPrediction, anyhow::Error> {
    let model_id = "au.anthropic.claude-sonnet-4-5-20250929-v1:0";
    info!(model_id = %model_id, market_type = %market_type, "Invoking Bedrock model");
    
    let odds_data_ref = odds_data.as_ref().map_or("", |s| s.as_str());
    let volume_data_ref = volume_data.as_ref().map_or("", |s| s.as_str());
    let news_context_ref = news_context.as_ref().map_or("", |s| s.as_str());
    
    let system_prompt = format!(
        "You are an expert prediction market analyst. Your task is to analyze the provided prediction market data and output your analysis as a single, raw JSON object. Do not add any text, explanations, or markdown formatting before or after the JSON object.
        CRITICAL INSTRUCTIONS:

        Identify and evaluate EVERY possible outcome in this prediction market (there may be 2, 3, 5, 10+ options, or it may be a binary YES/NO market)
        Your PRIMARY objective is to identify edge case or contrarian outcomes that are supported by strong primary data (polls, statistics, historical precedent, expert analysis, on-the-ground reporting, official data sources) — even if market sentiment disagrees
        Treat current market trading behavior (volume, odds, buy patterns) as LOW-WEIGHT signals only. They reflect crowd sentiment, not ground truth. Use them as a minor tiebreaker or sanity check, not as directional guidance
        Conduct deep comparative analysis across ALL outcomes, prioritizing outcomes where primary data diverges significantly from market pricing — this divergence itself is a signal of potential edge
        Your recommendation must reflect the outcome best supported by PRIMARY DATA, even if it is currently underpriced or unpopular in the market
        For binary markets: recommend YES if primary data supports the event occurring, NO if it doesn't — regardless of how the market is trading
        For multi-outcome markets: identify which specific outcome has the strongest primary data backing, then assess whether that outcome is genuinely probable or just relatively more likely among weak options
        Explicitly flag when your recommendation contradicts market consensus — this is often where the most valuable edge lies

        MAKE SURE YOU OUTPUT THE PREDICTION IN THE FOLLOWING JSON FORMAT AND NOTHING ELSE - DO NOT INCLUDE ANY EXPLANATION OR ADDITIONAL TEXT, JUST THE RAW JSON

        **JSON Output Format:**
        Your entire response must be a single JSON object with the following structure. Fill in the values based on your analysis.

        {{
            \"marketType\": \"Binary or Multi-outcome\",
            \"selectedOutcome\": \"The specific outcome you determined is most likely after evaluating ALL possibilities (for binary markets, state the event itself - i.e. name of selected individual or event)\",
            \"prediction\": \"Your concise paragraph explaining why this outcome is most probable compared to all alternatives in this market\",
            \"recommendedBuy\": \"YES or NO on the selectedOutcome - brief explanation why this represents the best position after comparing all available options\",
            \"confidence\": 0.75,
            \"dataVsMarketDivergence\": \"High / Medium / Low — how much does your primary-data-driven conclusion differ from current market pricing?\",
            \"keyFactors\": [\"Factor 1\", \"Factor 2\", \"Factor 3\"],
            \"marketSentimentNote\": \"Brief note on what the market currently favors and why you are or aren't following it\",
            \"risks\": [\"Risk 1\", \"Risk 2\", \"Risk 3\"],
            \"timeSensitivity\": \"Urgency level here\",
            \"alternativeOutcomesConsidered\": [\"List all other outcomes you evaluated and why each is less likely than your selected outcome\"]
}}"
    );

    let user_message = format!(
            "Provide a market prediction for the {} market based on current conditions.
            **Odds Data:**
            {}
            **Volume Data:**
            {}
            **News Context:**
            {}
            ",
            market_type, odds_data_ref, volume_data_ref, news_context_ref
        );

    let content = ContentBlock::Text(user_message);

    let message = aws_sdk_bedrockruntime::types::Message::builder()
        .role(aws_sdk_bedrockruntime::types::ConversationRole::User)
        .content(content)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build message: {}", e))?;
    
    info!("Sending request to Bedrock");

    let response = client
        .converse()
        .model_id(model_id)
        .system(aws_sdk_bedrockruntime::types::SystemContentBlock::Text(system_prompt))
        .messages(message)
        .send()
        .await;

    match response {
        Ok(output) => {
            info!("Received successful response from Bedrock");
            if let Some(ConverseOutput::Message(message)) = output.output {
                if let Some(ContentBlock::Text(text)) = message.content().first() {
                    debug!(response_text_len = text.len(), "Bedrock response text received");

                    // The model sometimes wraps the JSON in markdown or adds extra text.
                    // We'll find the first '{' and the last '}' to extract the JSON object.
                    let json_start = text.find('{');
                    let json_end = text.rfind('}');

                    if let (Some(start), Some(end)) = (json_start, json_end) {
                        let json_str = &text[start..=end];
                        debug!(json_str = %json_str, "Extracted JSON string from Bedrock response");

                        if let Ok(parsed) = serde_json::from_str::<BedrockPrediction>(json_str) {
                            let mut result = parsed;
                            result.confidence = result.confidence.clamp(0.0, 1.0);
                            info!("Successfully parsed Bedrock JSON response");
                            Ok(result)
                        } else {
                            warn!(response_text = %text, "Could not parse extracted JSON, using raw text as prediction");
                            Ok(BedrockPrediction {
                                market_type: "Unknown".to_string(),
                                selected_outcome: "Unknown".to_string(),
                                prediction: text.clone(),
                                recommended_buy: "No recommendation available".to_string(),
                                confidence: 0.5,
                                key_factors: Vec::new(),
                                risks: Vec::new(),
                                time_sensitivity: "Not specified".to_string(),
                                alternative_outcomes_considered: Vec::new(),
                                data_vs_market_divergence: "Not specified".to_string(),
                                market_sentiment_note: "Not specified".to_string(),
                            })
                        }
                    } else {
                        warn!(response_text = %text, "Bedrock response did not contain a JSON object, using raw text as prediction");
                        Ok(BedrockPrediction {
                            market_type: "Unknown".to_string(),
                            selected_outcome: "Unknown".to_string(),
                            prediction: text.clone(),
                            recommended_buy: "No recommendation available".to_string(),
                            confidence: 0.5,
                            key_factors: Vec::new(),
                            risks: Vec::new(),
                            time_sensitivity: "Not specified".to_string(),
                            alternative_outcomes_considered: Vec::new(),
                            data_vs_market_divergence: "Not specified".to_string(),
                            market_sentiment_note: "Not specified".to_string(),
                        })
                    }
                } else {
                    error!("No text content in Bedrock response");
                    Err(anyhow::anyhow!("No text content in Bedrock response"))
                }
            } else {
                error!("Unexpected response type from Bedrock");
                Err(anyhow::anyhow!("Unexpected response type from Bedrock"))
            }
        }
        Err(e) => {
            error!(error = %e, "Bedrock API error");
            Err(anyhow::anyhow!("Failed to invoke Bedrock model: {}", e))
        }
    }
}

#[instrument(skip(event))]
async fn handler(event: Request) -> Result<Response<Body>, Error> {
    info!("Lambda handler invoked");
    
    dotenv().ok(); // Reads the .env file
    info!("Environment variables loaded from .env file");

    let api_key = env::var("API_KEY").expect("API_KEY must be set");
    info!("API_KEY loaded successfully");
    
    // Check API key
    let headers = event.headers();
    let request_api_key = headers.get("x-api-key");
    info!(has_api_key = request_api_key.is_some(), "Checking API key");
    
    if request_api_key != Some(&api_key.parse().unwrap()) {
        warn!("Invalid or missing API key");
        return Ok(Response::builder().status(403).body("Forbidden".into())?);
    }
    info!("API key validated successfully");

    // Get a shared instance of the Bedrock client, initialized on first use.
    let bedrock_client = get_bedrock_client().await;

    match event.method() {
        m if *m == Method::POST => {
            info!("Handling POST request");
            let app_request: AppRequest = event.payload()?.unwrap_or_default();
            info!(market_url = %app_request.market_url, "Received request");

            if app_request.market_url.is_empty()
                || (!app_request.market_url.contains("kalshi.com/markets/") && !app_request.market_url.contains("polymarket.com/event/")) {
                warn!(market_url = %app_request.market_url, "Invalid market URL");
                return Ok(Response::builder()
                    .status(400)
                    .header("content-type", "application/json")
                    .body(r#"{"error":"Invalid request"}"#.into())
                    .expect("Failed to build error response"));
            }
            
            info!("Calling external Kalshi API");
            let (market_type, odds_data, volume_data, news_context) =
                match call_external_api(&app_request.market_url).await {
                    Ok(data) => {
                        info!(market_type = %data.0, "Successfully fetched market data");
                        data
                    }
                    Err(e) => {
                        error!(error = %e, "Error calling external API");
                        return Ok(Response::builder()
                            .status(500)
                            .header("content-type", "application/json")
                            .body(r#"{"error":"Failed to fetch market data"}"#.into())
                            .expect("Failed to build error response"));
                    }
                };

            let sanitised_market_data = match invoke_bedrock_sanitisation_model(&bedrock_client, &market_type, &odds_data, &volume_data, &news_context).await {
                Ok(data) => {
                    info!(market_type = %data.0, "Successfully sanitised market data");
                    data
                }
                Err(e) => {
                    error!(error = %e, "Error sanitising market data with Bedrock");
                    return Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(r#"{"error":"Failed to sanitise market data"}"#.into())
                        .expect("Failed to build error response"));
                }
            };

            // Invoke Bedrock model for prediction
            info!(market_type = %market_type, "Invoking Bedrock model for prediction");
            let prediction_result = match invoke_bedrock_prediction_model(
                &bedrock_client,
                &market_type,
                Some(sanitised_market_data.1),
                Some(sanitised_market_data.2),
                Some(sanitised_market_data.3),
            )
            .await {
                Ok(result) => {
                    info!(confidence = result.confidence, "Successfully generated prediction");
                    result
                }
                Err(e) => {
                    error!(error = %e, "Failed to invoke Bedrock model");
                    return Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(r#"{"error":"Failed to generate prediction"}"#.into())
                        .expect("Failed to build error response"));
                }
            };

            let app_response = AppResponse {
                market_type: prediction_result.market_type,
                selected_outcome: prediction_result.selected_outcome,
                prediction: prediction_result.prediction,
                recommended_buy: prediction_result.recommended_buy,
                confidence: prediction_result.confidence,
                data_vs_market_divergence: prediction_result.data_vs_market_divergence,
                market_sentiment_note: prediction_result.market_sentiment_note,
                key_factors: prediction_result.key_factors,
                risks: prediction_result.risks,
                time_sensitivity: prediction_result.time_sensitivity,
                alternative_outcomes_considered: prediction_result.alternative_outcomes_considered,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                model: "1.0.0".to_string(),
            };
            
            info!(timestamp = app_response.timestamp, "Building successful response");

            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&app_response)?.into())
                .expect("Failed to build response"))
        }
        m => {
            warn!(method = %m, "Method not allowed");
            Ok(Response::builder()
                .status(405)
                .body("Method Not Allowed".into())
                .expect("Failed to build response"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing subscriber for structured logging in CloudWatch
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .json()
        .init();
    
    info!("Lambda function starting up");
    lambda_http::run(service_fn(handler)).await
}
