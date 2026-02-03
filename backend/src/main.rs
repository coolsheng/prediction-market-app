use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    market_type: String,
}

#[derive(Serialize)]
struct Response {
    prediction: String,
    confidence: f32,
    timestamp: i64,
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let market_type = if event.payload.market_type.is_empty() {
        "general".to_string()
    } else {
        event.payload.market_type.clone()
    };

    // Simple prediction logic
    let prediction = match market_type.as_str() {
        "crypto" => "Crypto markets expected to be volatile",
        "stocks" => "Stock market showing bullish trends",
        "commodities" => "Commodities prices stabilizing",
        _ => "General market conditions are favorable",
    };

    let response = Response {
        prediction: prediction.to_string(),
        confidence: 0.75,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    };

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(function_handler);
    lambda_runtime::run(func).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_runtime::Context;

    #[tokio::test]
    async fn test_function_handler() {
        let request = Request {
            market_type: "crypto".to_string(),
        };
        
        let context = Context::default();
        let event = LambdaEvent::new(request, context);
        
        let response = function_handler(event).await.unwrap();
        assert!(response.prediction.contains("Crypto"));
        assert!(response.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_default_market_type() {
        let request = Request {
            market_type: String::new(),
        };
        
        let context = Context::default();
        let event = LambdaEvent::new(request, context);
        
        let response = function_handler(event).await.unwrap();
        assert!(response.prediction.contains("General"));
    }
}
