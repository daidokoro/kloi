#!/bin/bash

# Function to determine OS and ARCH
detect_os_arch() {
    OS=$(uname | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case $OS in
        darwin)
            OS="darwin"
            ;;
        linux)
            OS="linux"
            ;;
        *)
            echo "Unsupported OS: $OS"
            exit 1
            ;;
    esac

    case $ARCH in
        x86_64)
            ARCH="x86_64"
            ;;
        arm64 | aarch64)
            ARCH="aarch64"
            ;;
        *)
            echo "Unsupported architecture for this installation method: $ARCH"
            exit 1
            ;;
    esac

    echo "${OS}-${ARCH}"
}

# Function to download and install the correct version of kloi
install_kloi() {
    OS_ARCH=$(detect_os_arch)
    RELEASE_URL="https://github.com/daidokoro/kloi/releases/latest/download/kloi-${OS_ARCH}.tar.gz"

    # Download the correct binary
    echo "Downloading kloi for ${OS_ARCH}..."
    curl -L -o kloi.tar.gz $RELEASE_URL

    # Extract the binary
    echo "Installing kloi..."
    tar -xzf kloi.tar.gz
    chmod +x kloi

    # Move it to /usr/local/bin
    mv kloi /usr/local/bin/kloi

    # Cleanup
    rm kloi.tar.gz

    echo "kloi installed successfully!"
}

# Run the install function
install_kloi