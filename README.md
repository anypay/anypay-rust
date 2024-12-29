# WebSocket Event Server

A WebSocket server implementation in Rust that allows clients to establish persistent connections and subscribe to events through an internal message bus.

## Features
- WebSocket server using tokio and tokio-tungstenite
- Event dispatcher system with pub/sub capabilities
- Async message handling
- Client session management
- Support for multiple concurrent connections

## Prerequisites

- Rust (1.70.0 or later)
- Python 3.7+ (for running tests)
- pip (for installing Python dependencies)

## Building the Project

1. Clone the repository:

```bash
git clone <repository-url>
cd <project-directory>
```

2. Build the project:

```bash
cargo build --release
```

## Running the Server

Start the WebSocket server:

```bash
cargo run --release
```

The server will start listening on `ws://localhost:8080` by default.

## Running the Tests

1. First, install the required Python dependencies:

```bash
pip install websockets
```

2. Make sure the WebSocket server is running in one terminal:

```bash
cargo run --release
```

3. In another terminal, run the Python test script:

```bash
python3 scripts/test_deamon_comprehensive.py
```

The test script will run through several test cases:
- Basic subscription functionality
- Error handling
- Concurrent connections

## Test Script Structure

The test script (`scripts/test_websocket.py`) includes several test cases:

- `test_basic_functionality()`: Tests basic subscribe/unsubscribe operations
- `test_error_cases()`: Tests various error conditions
- `test_concurrent_connections()`: Tests multiple simultaneous connections

## Example WebSocket Messages

Subscribe to an event:

```json
{
    "action": "subscribe",
    "type": "invoice",
    "id": "inv_123"
}
```

Unsubscribe from an event:

```json
{
    "action": "unsubscribe",
    "type": "invoice",
    "id": "inv_123"
}
```

## Project Structure

```
.
├── src/
│   ├── main.rs           # Server implementation
│   ├── types.rs          # Message and type definitions
│   ├── session.rs        # Client session management
│   └── event_dispatcher.rs # Event subscription system
├── scripts/
│   └── test_websocket.py # Python test suite
└── README.md
```

## Contributing

1. Fork the repository
2. Create your feature branch (\`git checkout -b feature/amazing-feature\`)
3. Commit your changes (\`git commit -m 'Add some amazing feature'\`)
4. Push to the branch (\`git push origin feature/amazing-feature\`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
