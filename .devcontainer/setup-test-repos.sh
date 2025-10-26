#!/bin/bash
set -e

echo "Setting up test environment..."

# Create test environment directory
mkdir -p /tmp/testenv

# Install Jujutsu (jj) if not already installed
if ! command -v jj &> /dev/null; then
    echo "Installing Jujutsu (jj)..."

    # Fetch latest release assets and pick a Linux x86_64 tarball (gz or xz)
    RELEASE_JSON=$(curl -fsSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: devcontainer-setup' https://api.github.com/repos/martinvonz/jj/releases/latest || true)
    # Try tar archives first (gnu/musl)
    ASSET_URL=$(printf "%s" "$RELEASE_JSON" \
      | grep -Eo '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]+"' \
      | cut -d '"' -f 4 \
      | grep -Ei '(x86_64|amd64).*(unknown-linux-(gnu|musl)).*\.tar\.(gz|xz)$' \
      | head -n 1)

    # Fallback to .deb if no tarball found
    if [ -z "$ASSET_URL" ]; then
      ASSET_URL=$(printf "%s" "$RELEASE_JSON" \
        | grep -Eo '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]+"' \
        | cut -d '"' -f 4 \
        | grep -Ei '\.(deb)$' \
        | grep -Ei '(amd64|x86_64)' \
        | head -n 1)
      AS_DEB=1
    else
      AS_DEB=0
    fi

    if [ -z "$ASSET_URL" ]; then
        echo "Error: Could not find jj release asset for Linux x86_64"
        exit 1
    fi

    TMPDIR=$(mktemp -d)
    if [ "$AS_DEB" -eq 0 ]; then
      ARCHIVE="$TMPDIR/jj.tar"
      curl -fsSL "$ASSET_URL" -o "$ARCHIVE"

      case "$ASSET_URL" in
          *.tar.xz) tar -xJf "$ARCHIVE" -C "$TMPDIR" ;;
          *.tar.gz) tar -xzf "$ARCHIVE" -C "$TMPDIR" ;;
          *) echo "Error: Unknown archive type: $ASSET_URL"; rm -rf "$TMPDIR"; exit 1 ;;
      esac

      JJ_PATH=$(find "$TMPDIR" -type f -name jj -perm -u+x | head -n 1 || true)
      if [ -z "$JJ_PATH" ]; then
          echo "Error: jj binary not found inside archive"
          rm -rf "$TMPDIR"
          exit 1
      fi

      DEST="/usr/local/bin/jj"
      if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
          sudo install -m 0755 "$JJ_PATH" "$DEST"
      elif [ -w "$(dirname "$DEST")" ]; then
          install -m 0755 "$JJ_PATH" "$DEST"
      else
          mkdir -p "$HOME/.local/bin"
          install -m 0755 "$JJ_PATH" "$HOME/.local/bin/jj"
          if ! grep -q 'export PATH="$HOME/.local/bin:$PATH"' "$HOME/.bashrc" 2>/dev/null; then
              echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
          fi
          if ! grep -q 'export PATH="$HOME/.local/bin:$PATH"' "$HOME/.profile" 2>/dev/null; then
              echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
          fi
          export PATH="$HOME/.local/bin:$PATH"
      fi
    else
      DEB_FILE="$TMPDIR/jj.deb"
      curl -fsSL "$ASSET_URL" -o "$DEB_FILE"
      if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
        sudo dpkg -i "$DEB_FILE" || sudo apt-get update && sudo apt-get -y -f install
      else
        dpkg -i "$DEB_FILE" || (apt-get update && apt-get -y -f install)
      fi
    fi

    rm -rf "$TMPDIR"
    echo "Jujutsu installed successfully"
else
    echo "Jujutsu already installed"
fi

# Setup Git test repository
echo "Setting up git test repository..."
mkdir -p /tmp/testenv/git-repo
cd /tmp/testenv/git-repo

# Initialize git repository
git init
git config user.name "Test User"
git config user.email "test@example.com"

# Create initial files and commit
cat > README.md << 'EOF'
# Git Test Repository

This is a test repository for turboCommit testing.
EOF

mkdir -p src
cat > src/main.rs << 'EOF'
fn main() {
    println!("Hello, world!");
}
EOF

# Make initial commit
git add .
git commit -m "Initial commit: Add README and main.rs"

# Create staged changes for testing
echo "// New feature" >> src/main.rs
echo "fn new_function() {" >> src/main.rs
echo "    println!(\"New feature!\");" >> src/main.rs
echo "}" >> src/main.rs

git add src/main.rs

echo "Git repository setup complete with staged changes"

# Setup Jujutsu test repository
echo "Setting up jj test repository..."
mkdir -p /tmp/testenv/jj-repo
cd /tmp/testenv/jj-repo

# Initialize git repository first
git init
git config user.name "Test User"
git config user.email "test@example.com"

# Create initial files and commit
cat > README.md << 'EOF'
# Jujutsu Test Repository

This is a test repository for turboCommit jj integration testing.
EOF

mkdir -p src
cat > src/lib.rs << 'EOF'
pub fn hello() {
    println!("Hello from jj!");
}
EOF

# Make initial commit
git add .
git commit -m "Initial commit: Add README and lib.rs"

# Import into jj
jj git init --git-repo=.

# Create uncommitted changes for testing
echo "// Additional functionality" >> src/lib.rs
echo "pub fn goodbye() {" >> src/lib.rs
echo "    println!(\"Goodbye from jj!\");" >> src/lib.rs
echo "}" >> src/lib.rs

echo "Jujutsu repository setup complete with uncommitted changes"

# Verify installations
echo "Verifying installations..."
echo "Rust version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo "Git version: $(git --version)"
echo "Jujutsu version: $(jj --version)"

echo "Test environment setup complete!"
echo "Test repositories are available at:"
echo "  - Git repo: /tmp/testenv/git-repo"
echo "  - Jj repo: /tmp/testenv/jj-repo"
