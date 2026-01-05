#!/bin/bash
set -e

echo "🚀 Starting dlt2log installation..."

# 1. Check if Rust/Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "⚠️  Rust is not installed. Installing Rust via rustup..."
    # Install Rust using the official script in non-interactive mode (-y)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source the environment variables
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
    
    echo "✅ Rust installed successfully."
else
    echo "✅ Rust is already installed: $(cargo --version)"
fi

# 2. Install dlt2log from Crates.io
echo "📦 Installing dlt2log from Crates.io..."
if cargo install dlt2log; then
    echo ""
    echo "🎉 Installation complete!"
    
    # 3. Handle PATH automatically
    CARGO_BIN="$HOME/.cargo/bin"
    if [[ ":$PATH:" != *":$CARGO_BIN:"* ]]; then
        echo "⚙️  Adding $CARGO_BIN to PATH..."
        
        # Determine which shell config to use
        SHELL_TYPE=$(basename "$SHELL")
        CONFIG_FILE=""
        
        if [ "$SHELL_TYPE" == "zsh" ]; then
            CONFIG_FILE="$HOME/.zshrc"
        elif [ "$SHELL_TYPE" == "bash" ]; then
            if [ -f "$HOME/.bash_profile" ]; then
                CONFIG_FILE="$HOME/.bash_profile"
            else
                CONFIG_FILE="$HOME/.bashrc"
            fi
        fi
        
        if [ -n "$CONFIG_FILE" ]; then
            if ! grep -q "$CARGO_BIN" "$CONFIG_FILE" 2>/dev/null; then
                echo "" >> "$CONFIG_FILE"
                echo "# Added by dlt2log installer" >> "$CONFIG_FILE"
                echo "export PATH=\"\$HOME/.cargo/bin:\$PATH\"" >> "$CONFIG_FILE"
                echo "✅ Added PATH to $CONFIG_FILE"
                echo "💡 Please run 'source $CONFIG_FILE' or restart your terminal to use 'dlt2log'."
            fi
        else
            echo "⚠️  Could not detect shell config file. Please add $CARGO_BIN to your PATH manually."
        fi
    else
        echo "✅ $CARGO_BIN is already in PATH."
        echo "🚀 You can now run 'dlt2log --help' from anywhere."
    fi
else
    echo "❌ Installation failed."
    exit 1
fi
