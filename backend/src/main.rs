pub mod models;

use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::types::ContentBlock;
use aws_sdk_bedrockruntime::types::ConverseOutput;
use dotenv::dotenv;
use http::Method;
use lambda_http::{service_fn, Body, Error, Request, RequestPayloadExt, Response};
use models::KelshiContext;
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
    prediction: String,
    recommended_buy: String,
    confidence: f32,
    key_factors: Vec<String>,
    risks: Vec<String>,
    time_sensitivity: String,
    timestamp: i64,
    model: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BedrockPrediction {
    prediction: String,
    #[serde(default)]
    recommended_buy: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    key_factors: Vec<String>,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(default = "default_time_sensitivity")]
    time_sensitivity: String,
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
    info!(market_url = %url, "Calling external Kalshi API");
    
    // Regex to capture the market ticker from the URL and convert it to uppercase.
    let re = Regex::new(r"markets/([^/]+)")?;
    let series_ticker = re
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_uppercase())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract market ticker from URL: {}", url))?;
    
    info!(series_ticker = %series_ticker, "Extracted series ticker from URL");

    let client = reqwest::Client::new();
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
    let odds_data = market_data_as_string.clone();
    let volume_data = market_data_as_string;
    let news_context = String::new();
    
    info!(market_type = %market_type, "Extracted market data successfully");

    Ok((market_type, odds_data, volume_data, news_context))
}

/// Invoke AWS Bedrock model to generate a prediction
#[instrument(skip(client, odds_data, volume_data, news_context))]
async fn invoke_bedrock_model(
    client: &BedrockClient,
    market_type: &str,
    odds_data: Option<String>,
    volume_data: Option<String>,
    news_context: Option<String>,
) -> Result<(String, String, f32, Vec<String>, Vec<String>, String), anyhow::Error> {
    let model_id = "anthropic.claude-3-haiku-20240307-v1:0";
    info!(model_id = %model_id, market_type = %market_type, "Invoking Bedrock model");
    
    let odds_data_ref = odds_data.as_ref().map_or("", |s| s.as_str());
    let volume_data_ref = volume_data.as_ref().map_or("", |s| s.as_str());
    let news_context_ref = news_context.as_ref().map_or("", |s| s.as_str());
    
    debug!(odds_data_len = odds_data_ref.len(), volume_data_len = volume_data_ref.len(), news_context_len = news_context_ref.len(), "Input data sizes");

    let system_prompt = format!(
                "Role: Expert prediction market analyst
                Task: Analyze {}
                Data: [ODDS_DATA] {}, [VOLUME_DATA] {}, [NEWS_CONTEXT] {}
                MAKE SURE YOU OUTPUT THE PREDICTION IN THE FOLLOWING JSON FORMAT AND NOTHING ELSE - DO NOT INCLUDE ANY EXPLANATION OR ADDITIONAL TEXT, JUST THE RAW JSON
                Output format:
                {{
                    \"prediction\": \"Your concise prediction here - make sure it's just a paragraph for explaination \",
                    // a recommendation on what to buy (YES or NO) with a brief explanation of why
                    \"recommendedBuy\": \"YES or NO - brief explanation why\",
                    // a confidence score between 0 and 1 indicating how confident you are in the prediction - this should be based on the data and your analysis, not a generic statement
                    \"confidence\": 0.XX\",
                    // list of key factors influencing the market - these should be specific to the market and data provided, not generic factors
                    \"keyFactors\": [\"Factor 1\", \"Factor 2\", \"Factor 3\"],
                    // list of specific risks or uncertainties that could impact the market outcome - again, these should be based on the data and market context, not generic risks
                    \"risks\": [\"Risk 1\",\"Risk 2\", \"Risk 3\"],
                    // an assessment of how time-sensitive the prediction is - this should be based on the current market conditions and data, not a generic statement
                    \"timeSensitivity\": \"Urgency level here\",
                }}",
        market_type, odds_data_ref, volume_data_ref, news_context_ref
    );

    let user_message = format!(
            "Provide a market prediction for the {} market based on current conditions.",
            market_type
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

                    if let Ok(parsed) = serde_json::from_str::<BedrockPrediction>(text) {
                        // info!(confidence = parsed.confidence, key_factors_count = parsed.key_factors.len(), risks_count = parsed.risks.len(), "Successfully parsed Bedrock response");
                        Ok((
                            parsed.prediction,
                            parsed.recommended_buy,
                            parsed.confidence.clamp(0.0, 1.0),
                            parsed.key_factors,
                            parsed.risks,
                            parsed.time_sensitivity,
                        ))
                    } else {
                        warn!(response_text = %text, "Bedrock response was not valid JSON, using raw text");
                        Ok((text.clone(), "No recommendation available".to_string(), 0.5, Vec::new(), Vec::new(), "Not specified".to_string()))
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

            if app_request.market_url.is_empty() || !app_request.market_url.contains("kalshi.com/markets/") {
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

            // Invoke Bedrock model for prediction
            info!(market_type = %market_type, "Invoking Bedrock model for prediction");
            let (prediction, recommended_buy, confidence, key_factors, risks, time_sensitivity) = match invoke_bedrock_model(
                &bedrock_client,
                &market_type,
                Some(odds_data),
                Some(volume_data),
                Some(news_context),
            )
            .await {
                Ok(result) => {
                    info!(confidence = result.1, "Successfully generated prediction");
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
                prediction,
                recommended_buy,
                confidence,
                key_factors,
                risks,
                time_sensitivity,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                model: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
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
