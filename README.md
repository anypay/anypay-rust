![Anypay Logo](https://bico.media/4f913a35258626de7e07571b0ef8de39e9e77908570a4a4ae2af6072bb34a59d)

# Anypay Rust SDK 🚀

Welcome to Anypay's WebSocket Tools! This powerful suite enables real-time payment processing and monitoring through a modern WebSocket interface. Built with Rust for maximum performance and reliability, these tools make cryptocurrency payment integration a breeze! 💫

## What's Inside? 📦

- **`anypay-client`** 🔧: A powerful CLI tool for creating and managing invoices, submitting payments, and monitoring payment status in real-time
- **`anypay-server`** 🖥️: A high-performance WebSocket server that handles payment processing and real-time notifications

## Features ✨

- **Real-time Updates** 🔄: Get instant notifications about payment status changes
- **Multi-Currency Support** 💰: Handle payments in various cryptocurrencies
- **Secure Authentication** 🔒: Built-in token-based security
- **Flexible Integration** 🔌: Easy-to-use CLI and WebSocket interfaces
- **Automatic Payment Options** ⚡: Smart payment option generation based on current prices

## Installation 🛠️

### From crates.io
```bash
# Install both client and server binaries
cargo install anypay

# Or install them separately
cargo install anypay-client
cargo install anypay-server
```

### From Source
```bash
# Clone the repository
git clone https://github.com/anypay/anypay
cd anypay-websockets-rust

# Build the release binaries
cargo build --release

# The binaries will be available in target/release/
```

## anypay-client Usage 🔧

### Authentication 🔑
Provide your API token either:
- As a command line argument: `--token YOUR_TOKEN`
- Via environment variable: `export ANYPAY_TOKEN=YOUR_TOKEN`

### Available Commands 💻

#### Create an Invoice 📝
```bash
anypay-client create-invoice \
  --amount 100 \
  --currency USD \
  --webhook https://example.com/webhook \
  --redirect https://example.com/return \
  --memo "Payment for services"
```

#### Request a Payment 💸
```bash
anypay-client request-payment \
  --currency BTC \
  --address bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh \
  --amount 100 \
  --quote-currency USD
```

#### Submit a Payment 📤
```bash
anypay-client submit-payment \
  --uid inv_123 \
  --chain BTC \
  --currency BTC \
  --txhex 0200000001...
```

#### Get Invoice Details 📋
```bash
anypay-client get-invoice inv_123
```

#### Cancel an Invoice ❌
```bash
anypay-client cancel-invoice inv_123
```

#### Get Current Prices 📊
```bash
anypay-client get-prices
```

#### Monitor Invoice Updates 👀
```bash
anypay-client monitor inv_123
```

### Additional Options ⚙️

- `--json`: Output responses in JSON format
- `--endpoint URL`: Use a custom API endpoint
- `--debug`: Enable debug logging

## anypay-server Usage 🖥️

### Configuration ⚙️
Configure the server using environment variables:

```bash
# Required 🔒
export SUPABASE_URL=your_supabase_url
export SUPABASE_KEY=your_supabase_key

# Optional 🔧
export PORT=8080  # Default: 8080
export HOST=0.0.0.0  # Default: 0.0.0.0
export LOG_LEVEL=debug  # Default: info
```

### Running the Server 🚀
```bash
# Start the server
anypay-server

# With custom port
anypay-server --port 9000

# With debug logging
anypay-server --debug
```

### Server Features 🌟
- Real-time WebSocket communication 🔄
- Price updates and conversions 💱
- Invoice creation and management 📋
- Payment processing 💳
- Event subscriptions 📡
- Automatic payment option generation ⚡

## Development 👩‍💻

### Requirements 📋
- Rust 1.70 or later
- Cargo package manager

### Building 🏗️
```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Testing 🧪
```bash
cargo test
```

## API Documentation 📚

For detailed API documentation, including WebSocket message formats and HTTP endpoints, see [API.md](API.md).

## Get Started Today! 🚀

Start accepting cryptocurrency payments in minutes with Anypay's WebSocket tools. For more information, visit our [documentation](https://docs.anypay.com) or contact our [support team](mailto:support@anypay.com).

## License 📜

MIT License. See [LICENSE](LICENSE) for details.

---

Thank you for choosing Anypay! We look forward to powering your payment solutions. 😊

# Homebrew Anypay

Homebrew tap for Anypay payment processing tools.

## Installation

Add the tap:
```bash
brew tap anypay/anypay
```

Install all tools:
```bash
brew install anypay
```

Or install individual components:
```bash
brew install anypay-server
brew install anypay-wallet
```

## Available Formulae

- `anypay` - Complete suite
- `anypay-server` - Payment processing server
- `anypay-wallet` - Wallet client