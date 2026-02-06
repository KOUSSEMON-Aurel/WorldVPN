#!/bin/bash
set -e

# Configuration
REMOTE_USER="aurel"
REMOTE_HOST="dexgunaurel.duckdns.org"
REMOTE_BUILD_DIR="~/worldvpn-build"
# Note: Password is theoretically handled by sshpass if installed, or interactive prompt.
# The user provided password: S1mph0n1_aurel.
# CAUTION: Storing passwords in scripts is insecure. Ideally use SSH keys.
# We will use sshpass if available and SSHPASS env var is set, otherwise interactive.

echo "🚀 Starting Remote Build on ${REMOTE_USER}@${REMOTE_HOST}..."


# 1. Sync Source Code
echo "Syncing source code to remote..."
sshpass -p "$SSHPASS" rsync -avz --exclude 'target' --exclude 'node_modules' --exclude '.git' \
    -e "ssh -o StrictHostKeyChecking=no" \
    ./ ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_BUILD_DIR}/

# 2. Execute Build on Remote
echo "Executing build on remote machine..."
sshpass -p "$SSHPASS" ssh -o StrictHostKeyChecking=no ${REMOTE_USER}@${REMOTE_HOST} << EOF

    set -e
    cd ${REMOTE_BUILD_DIR}
    
    # Make sure scripts are executable
    chmod +x build-windows-docker.sh
    
    # Run the build script
    # The user mentioned a docker image: worldvpn-builder-windows:latest
    # If the build script uses 'cross', it handles docker images itself.
    # But if we need to use a SPECIFIC image, we might need to adjust build-windows-docker.sh
    # For now, let's run the corrected build script we made.
    
    ./build-windows-docker.sh
EOF

# 3. Retrieve Artifacts
echo "Retrieving artifacts..."
mkdir -p build-output
sshpass -p "$SSHPASS" scp -r ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_BUILD_DIR}/frontend/worldvpn-gui/src-tauri/target/x86_64-pc-windows-gnu/release/worldvpn-gui.exe ./build-output/
sshpass -p "$SSHPASS" scp -r ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_BUILD_DIR}/frontend/worldvpn-gui/src-tauri/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll ./build-output/

echo "✅ Build complete! Files are in ./build-output/"
