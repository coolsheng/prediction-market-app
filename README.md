# Prediction Market App

A monorepo containing a mobile application built with Expo and a serverless backend written in Rust.

## Project Structure

```
prediction-market-app/
├── mobile/           # Expo React Native mobile application
│   ├── src/
│   ├── assets/
│   ├── App.tsx
│   └── package.json
├── backend/          # Rust serverless backend for AWS Lambda
│   ├── src/
│   │   └── main.rs
│   └── Cargo.toml
└── package.json      # Root package.json for monorepo management
```

## Getting Started

### Prerequisites

- Node.js (v18 or higher)
- npm or yarn
- Rust (latest stable)
- Cargo
- Expo CLI (optional, for mobile development)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/coolsheng/prediction-market-app.git
   cd prediction-market-app
   ```

2. Install all dependencies:
   ```bash
   npm run install:all
   ```

   Or install individually:
   ```bash
   # Install mobile dependencies
   npm run mobile:install
   
   # Rust dependencies are managed by Cargo
   ```

## Mobile App

The mobile application is built with Expo and React Native, providing cross-platform support for iOS, Android, and Web.

### Running the Mobile App

```bash
npm run mobile
```

Or directly:
```bash
cd mobile
npm start
```

See [mobile/README.md](mobile/README.md) for more details.

## Backend

The backend is a serverless Rust application designed to run on AWS Lambda, providing prediction market logic.

### Building the Backend

```bash
npm run backend:build
```

Or directly:
```bash
cd backend
cargo build
```

### Testing the Backend

```bash
npm run backend:test
```

Or directly:
```bash
cd backend
cargo test
```

See [backend/README.md](backend/README.md) for deployment instructions.

## Tech Stack

### Mobile
- **Expo** - Development framework
- **React Native** - Cross-platform mobile framework
- **TypeScript** - Type-safe JavaScript

### Backend
- **Rust** - Systems programming language
- **AWS Lambda Runtime** - Serverless function execution
- **Tokio** - Async runtime
- **Serde** - Serialization framework

## Development

- Mobile app runs on Expo development server
- Backend can be tested locally with `cargo test`
- Both components are independent and can be developed separately

## License

MIT
