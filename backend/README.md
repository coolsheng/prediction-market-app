# Prediction Market Backend

A serverless Rust backend for the prediction market application, designed to run on AWS Lambda.

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Building for Lambda

For deployment to AWS Lambda, you need to build for the correct target:

```bash
# Build for Lambda
cargo lambda build --release
```

### Deploy
```
cargo lambda deploy
```


## API

## Deployment

This function is designed to be deployed as an AWS Lambda function using AWS SAM, Serverless Framework, or direct Lambda deployment.

### Using AWS SAM

1. Create a `template.yaml` SAM template
2. Package the function: `sam package`
3. Deploy: `sam deploy`

### Direct Lambda Deployment

1. Build the binary as described above
2. Create a deployment package (ZIP)
3. Upload to AWS Lambda

## Tech Stack

- Rust 2021 Edition
- AWS Lambda Runtime
- Tokio for async runtime
- Serde for JSON serialization
