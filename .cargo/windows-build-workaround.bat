#!/bin/bash
# Shell script to patch zstd-sys behavior

# Set env vars that should disable pkg-config
set -gx ZSTD_SYS_USE_PKG_CONFIG "0"
set -gx LIBZSTD_NO_PKG_CONFIG "1"
set -gx ZSTD_SYS_STATIC "1"

# Force pkg-config to be unavailable by adding an empty dummy
if [ ! -x "$(command -v pkg-config)" ]; then
    echo "pkg-config not found, proceeding with source compilation" >&2
    # Ensure our environment variables are exported
    export ZSTD_SYS_USE_PKG_CONFIG=0
    export LIBZSTD_NO_PKG_CONFIG=1
    export ZSTD_SYS_STATIC=1
    # Exit successfully for build script
    exit 0
fi

# Patching failsafe: if pkg-config exists, we must manually block zstd-sys
# Create a wrapper that always fails
CAT="/usr/bin/cat"
if command -v cat >/dev/null 2>&1; then
    # Create a mock cat that would cause PKG_CONFIG to fail
    export PATH="$PATH:$(pwd)"
    cat > zstd <<'EOF'
#!/bin/bash
echo "Mock pkg-config - returning failure" >&2
exit 1
EOF
    chmod +x zstd
    export PATH="$(pwd):$PATH"
    echo "Created mock pkg-config to simulate missing command" >&2
fi
