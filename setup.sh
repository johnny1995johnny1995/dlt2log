#!/bin/bash
set -e

echo "🚀 Starting dlt2log environment setup..."

# 1. Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo is not installed."
    echo "   Please install it by running: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "   Or visit https://rustup.rs/"
    exit 1
else
    echo "✅ Rust/Cargo is installed: $(cargo --version)"
fi

# 2. Build the project
echo "📦 Building project..."
cargo build --release

# 3. Final check
if [ -f "target/release/dlt2log" ]; then
    echo "✅ Build successful!"
    echo "   Binary location: ./target/release/dlt2log"
    echo "   Try running: ./target/release/dlt2log --help"
else
    echo "❌ Build failed."
    exit 1
fi

echo "🎉 Setup complete! You can now use 'make' to run commands."
