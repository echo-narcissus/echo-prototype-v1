BIN_DIR = ./bin
KEY_DIR = ./keys

BINARY_NAME = echo-server
KEY_FILE = $(KEY_DIR)/key.pem
CERT_FILE = $(KEY_DIR)/cert.pem

.PHONY: build tls setup start_default clean

build:
	@echo "Building echo..."
	@cd echo && cargo build -r
	@pwd
	@mkdir -p $(BIN_DIR)
	@cp "./echo/target/release/echo" $(BIN_DIR)
	@cd $(BIN_DIR) && mv echo $(BINARY_NAME)
	@echo "Build complete. Binary at $(BIN_DIR)/$(BINARY_NAME)"

tls:
	@echo "Generating TLS keys..."
	@mkdir -p $(KEY_DIR)
	@OPENSSL_CONF=/etc/ssl/openssl.cnf \
	openssl req -x509 -newkey rsa:2048 -nodes \
	  -keyout $(KEY_FILE) -out $(CERT_FILE) -days 365 \
	  -subj "/CN=echo-server"
	@echo "Keys generated in $(KEY_DIR)/"

setup: build tls
	@echo "Setup complete."

start_default:
	@if [ ! -f $(BIN_DIR)/$(BINARY_NAME) ]; then \
		echo "Binary not found. Running build..."; \
		$(MAKE) build; \
	fi
	@if [ ! -f $(KEY_FILE) ] || [ ! -f $(CERT_FILE) ]; then \
		echo "Keys not found. Running tls..."; \
		$(MAKE) tls; \
	fi
	@echo "Starting default server..."
	@$(BIN_DIR)/$(BINARY_NAME) -k $(KEY_FILE) -c $(CERT_FILE)

start_tiny:
	@if [ ! -f $(BIN_DIR)/$(BINARY_NAME) ]; then \
		echo "Binary not found. Running build..."; \
		$(MAKE) build; \
	fi
	@if [ ! -f $(KEY_FILE) ] || [ ! -f $(CERT_FILE) ]; then \
		echo "Keys not found. Running tls..."; \
		$(MAKE) tls; \
	fi
	@echo "Starting server with 1 byte message ID length..."
	@$(BIN_DIR)/$(BINARY_NAME) -k $(KEY_FILE) -c $(CERT_FILE) -m 1

clean:
	@echo "Cleaning build artifacts and keys..."
	@rm -rf $(BIN_DIR) $(KEY_DIR)
	@cd echo && cargo clean

soft_clean:
	@echo "Cleaning keys and deleting binary"
	@rm -rf $(BIN_DIR) $(KEY_DIR)

