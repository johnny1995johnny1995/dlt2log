.PHONY: all build run verify test clean help

# Default target
all: build

# Build the project in release mode
build:
	@echo "📦 Building..."
	cargo build --release

# Run on the verbose sample file (log_166.dlt)
run:
	cargo run -- dlt_v1_v2/log_166.dlt -o log_166.log

# Run batch verification on all files
verify:
	@echo "🔍 Running batch verification..."
	@mkdir -p output
	@for f in dlt_v1_v2/*.dlt; do \
		echo "Processing $$f..."; \
		cargo run --release -- "$$f" -o "$${f%.dlt}.log"; \
	done
	@echo "✅ Verification complete. Check .log files in dlt_v1_v2/"

# Run unit tests (if any)
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -f dlt_v1_v2/*.log
	rm -f *.log

# Show help
help:
	@echo "Available commands:"
	@echo "  make build   - Build binary (release mode)"
	@echo "  make run     - Run on sample log_166.dlt"
	@echo "  make verify  - Process all DLT files in dlt_v1_v2/"
	@echo "  make clean   - Remove build artifacts and logs"
