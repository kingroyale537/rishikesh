#!/usr/bin/env bash
# ==============================================================================
# Rishikesh Language Universal Installer (macOS, Linux, WSL)
# Usage: curl -fsSL https://rishikesh.lang/install.sh | sh
# ==============================================================================

set -e

RESET="\033[0m"
BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"

printf "${CYAN}${BOLD}"
cat << "EOF"
  ____  _     _     _ _              _     
 |  _ \(_)___| |__ (_) | _____  ___ | |__  
 | |_) | / __| '_ \| | |/ / _ \/ __|| '_ \ 
 |  _ <| \__ \ | | | |   <  __/\__ \| | | |
 |_| \_\_|___/_| |_|_|_|\_\___||___/|_| |_|
EOF
printf "${RESET}\n"
printf "${BOLD}The Universal Programming Language for AI, Systems, and Cloud${RESET}\n\n"

# 1. Detect Operating System & Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        TARGET_OS="macos"
        ;;
    Linux)
        TARGET_OS="linux"
        ;;
    *)
        printf "${RED}Error: Unsupported operating system: %s${RESET}\n" "$OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    arm64|aarch64)
        TARGET_ARCH="arm64"
        ;;
    *)
        printf "${RED}Error: Unsupported architecture: %s${RESET}\n" "$ARCH"
        exit 1
        ;;
esac

TARGET_TRIPLE="${TARGET_OS}-${TARGET_ARCH}"
printf "${GREEN}✓${RESET} Detected Platform: ${BOLD}%s${RESET}\n" "$TARGET_TRIPLE"

# 2. Installation Directory Setup
INSTALL_DIR="$HOME/.rishi/bin"
mkdir -p "$INSTALL_DIR"

LOCAL_BIN="$(which rishi 2>/dev/null || true)"
if [ -n "$LOCAL_BIN" ] && [ -f "$LOCAL_BIN" ]; then
    printf "${GREEN}✓${RESET} Using local toolchain binary: %s\n" "$LOCAL_BIN"
    cp "$LOCAL_BIN" "$INSTALL_DIR/rishi"
else
    # Fallback to downloading or cargo build
    printf "${YELLOW}→${RESET} Provisioning Rishikesh runtime...\n"
    if command -v cargo >/dev/null 2>&1; then
        cargo install --path "$(dirname "$0")/crates/rishi_cli" --root "$HOME/.rishi" --quiet
    else
        printf "${RED}Error: Neither prebuilt binary nor cargo found.${RESET}\n"
        exit 1
    fi
fi

chmod +x "$INSTALL_DIR/rishi"
printf "${GREEN}✓${RESET} Installed executable to: ${BOLD}%s/rishi${RESET}\n" "$INSTALL_DIR"

# 3. Configure Shell Environment ($PATH)
SHELL_CONFIG=""
case "$SHELL" in
    */zsh)
        SHELL_CONFIG="$HOME/.zshrc"
        ;;
    */bash)
        if [ -f "$HOME/.bashrc" ]; then
            SHELL_CONFIG="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            SHELL_CONFIG="$HOME/.bash_profile"
        fi
        ;;
    */fish)
        SHELL_CONFIG="$HOME/.config/fish/config.fish"
        ;;
esac

if [ -n "$SHELL_CONFIG" ] && [ -f "$SHELL_CONFIG" ]; then
    if ! grep -q ".rishi/bin" "$SHELL_CONFIG"; then
        printf "\n# Rishikesh Universal Language\nexport PATH=\"\$HOME/.rishi/bin:\$PATH\"\n" >> "$SHELL_CONFIG"
        printf "${GREEN}✓${RESET} Added \$HOME/.rishi/bin to ${BOLD}%s${RESET}\n" "$SHELL_CONFIG"
    else
        printf "${GREEN}✓${RESET} \$PATH already configured in %s\n" "$SHELL_CONFIG"
    fi
fi

# 4. Install VS Code Extension if code command is available
if command -v code >/dev/null 2>&1; then
    VSCODE_EXT_DIR="$(dirname "$0")/editors/vscode-rishikesh"
    if [ -d "$VSCODE_EXT_DIR" ]; then
        DEST_DIR="$HOME/.vscode/extensions/rishikesh-lang"
        mkdir -p "$DEST_DIR"
        cp -R "$VSCODE_EXT_DIR/"* "$DEST_DIR/"
        printf "${GREEN}✓${RESET} VS Code Rishikesh extension installed automatically!\n"
    fi
fi

printf "\n${GREEN}${BOLD}🎉 Rishikesh Language installed successfully!${RESET}\n\n"
printf "Get started by running:\n"
printf "  ${CYAN}rishi --help${RESET}         # View available commands\n"
printf "  ${CYAN}rishi repl${RESET}           # Start interactive REPL\n"
printf "  ${CYAN}rishi run hello.rk${RESET}   # Execute a Rishikesh script\n\n"
