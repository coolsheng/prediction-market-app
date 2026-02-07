pub mod models;

use std::time::SystemTime;
use lambda_http::{service_fn, Body, Error, Request, RequestPayloadExt, Response};
use serde::{Deserialize, Serialize};
use http::Method;
use regex::Regex;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::types::ContentBlock;
use aws_sdk_bedrockruntime::types::ConverseOutput;
use models::KelshiContext;
use reqwest;

#[derive(Deserialize, Default)]
struct AppRequest {
    #[serde(default)]
    market_url: String,
}

#[derive(Serialize)]
struct AppResponse {
    prediction: String,
    confidence: f32,
    key_factors: Vec<String>,
    risks: Vec<String>,
    time_sensitivity: String,
    timestamp: i64,
    model: String,
}

/// Initialize AWS Bedrock client
async fn get_bedrock_client() -> BedrockClient {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    BedrockClient::new(&config)
}

async fn call_external_api(url: &str) -> Result<(String, String, String, String), anyhow::Error> {
    // Regex to capture the market ticker from the URL and convert it to uppercase.
    let re = Regex::new(r"markets/([^/]+)")?;
    let series_ticker = re
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_uppercase())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract market ticker from URL: {}", url))?;

    let client = reqwest::Client::new();
    let api_url = format!("https://api.elections.kalshi.com/trade-api/v2/markets?series_ticker={}&status=open", series_ticker);

    let response = client.get(&api_url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Kalshi API request failed with status: {}", response.status()));
    }

    let kalshi_context: KelshiContext = serde_json::from_str(&response.text().await?)?;

    let market_type = kalshi_context.markets.first().map(|m| m.market_type.clone()).unwrap_or_default();
    let market_data_as_string = serde_json::to_string(&kalshi_context.markets)?;
    // right now - we're not differentiating between odds and volume data, but this can be easily extended in the future if needed by parsing the market data more granularly
    // also we're not getting any news context for now, but this can be added in the future by integrating with a news API and fetching relevant news based on the market's event ticker or other metadata
    let odds_data = market_data_as_string.clone();
    let volume_data = market_data_as_string;
    let news_context = String::new();

    Ok((market_type, odds_data, volume_data, news_context))
}

/// Invoke AWS Bedrock model to generate a prediction
async fn invoke_bedrock_model(
    client: &BedrockClient,
    market_type: &str,
    odds_data: Option<String>,
    volume_data: Option<String>,
    news_context: Option<String>,
) -> Result<(String, f32, Vec<String>, Vec<String>, String), anyhow::Error> {
    // Use Claude 3 Haiku model (fast and cost-effective)
    let model_id = "anthropic.claude-3-haiku-20240307-v1:0";
    
    let odds_data_ref = odds_data.as_ref().map_or("", |s| s.as_str());
    let volume_data_ref = volume_data.as_ref().map_or("", |s| s.as_str());
    let news_context_ref = news_context.as_ref().map_or("", |s| s.as_str());

    let system_prompt = format!(
                        "Role: Expert prediction market analyst
                Task: Analyze {}
                Data: [ODDS_DATA] {}, [VOLUME_DATA] {}, [NEWS_CONTEXT] {}
                Output format: 
                {{
                    \"prediction\": \"Your concise prediction here - make sure it's just a paragraph for explaination \",
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

    let response = client
        .converse()
        .model_id(model_id)
        .system(aws_sdk_bedrockruntime::types::SystemContentBlock::Text(system_prompt))
        .messages(message)
        .send()
        .await;

    match response {
        Ok(output) => {
            if let Some(ConverseOutput::Message(message)) = output.output {
                if let Some(ContentBlock::Text(text)) = message.content().first() {
                    // Try to parse the JSON response
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(text) {
                        let prediction = json_val
                            .get("prediction")
                            .and_then(|p| p.as_str())
                            .unwrap_or("Unable to parse prediction")
                            .to_string();
                        let confidence = json_val
                            .get("confidence")
                            .and_then(|c| c.as_f64())
                            .unwrap_or(0.5) as f32;
                        let key_factors = json_val
                            .get("keyFactors")
                            .and_then(|kf| kf.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect::<Vec<String>>()
                            })
                            .unwrap_or_default();
                        let risks = json_val
                            .get("risks")
                            .and_then(|r| r.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect::<Vec<String>>()
                            })
                            .unwrap_or_default();
                        let time_sensitivity = json_val
                            .get("timeSensitivity")
                            .and_then(|ts| ts.as_str())
                            .unwrap_or("Not specified")
                            .to_string();

                        Ok((prediction, confidence.clamp(0.0, 1.0), key_factors, risks, time_sensitivity))
                    } else {
                        // If not valid JSON, use the raw text
                        Ok((text.clone(), 0.5, Vec::new(), Vec::new(), "Not specified".to_string()))
                    }
                } else {
                    Err(anyhow::anyhow!("No text content in Bedrock response"))
                }
            } else {
                Err(anyhow::anyhow!("Unexpected response type from Bedrock"))
            }
        }
        Err(e) => {
            eprintln!("Bedrock API error: {:?}", e);
            Err(anyhow::anyhow!("Failed to invoke Bedrock model: {}", e))
        }
    }
}

async fn handler(event: Request) -> Result<Response<Body>, Error> {
    // Initialize Bedrock client
    let bedrock_client = get_bedrock_client().await;

    match *event.method() {
        Method::POST => {
            let app_request: AppRequest = event.payload()?.unwrap_or_default();

            if app_request.market_url.is_empty() || !app_request.market_url.contains("kalshi.com/markets/") {
                        return Ok(Response::builder()
                            .status(400)
                            .header("content-type", "application/json")
                            .body(r#"{"error":"Invalid request"}"#.into())
                            .expect("Failed to build error response"));            }
 
            let (market_type, odds_data, volume_data, news_context) =
                match call_external_api(&app_request.market_url).await {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Error calling external API: {}", e);
                        return Ok(Response::builder()
                            .status(500)
                            .header("content-type", "application/json")
                            .body(r#"{"error":"Failed to fetch market data"}"#.into())
                            .expect("Failed to build error response"));
                    }
                };

            // Invoke Bedrock model for prediction
            let (prediction, confidence, key_factors, risks, time_sensitivity) = match invoke_bedrock_model(
                &bedrock_client,
                &market_type,
                Some(odds_data),
                Some(volume_data),
                Some(news_context),
            )
            .await {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Failed to invoke Bedrock model: {}", e);
                    return Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(r#"{"error":"Failed to generate prediction"}"#.into())
                        .expect("Failed to build error response"));
                }
            };

            let app_response = AppResponse {
                prediction,
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

            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&app_response)?.into())
                .expect("Failed to build response"))
        }
        _ => Ok(Response::builder()
            .status(405)
            .body("Method Not Allowed".into())
            .expect("Failed to build response")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_http::run(service_fn(handler)).await
}
