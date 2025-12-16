#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

echo "======================================"
echo "Uploading Configuration File"
echo "======================================"

# Resolve config file path
CONFIG_PATH="${SCRIPT_DIR}/${CONFIG_FILE_PATH}"

# Check if config file exists
if [ ! -f "$CONFIG_PATH" ]; then
    echo "Error: Config file not found at: $CONFIG_PATH"
    exit 1
fi

echo "Config file: $CONFIG_PATH"

# Get storage account key
echo ""
echo "Retrieving storage account key..."
STORAGE_KEY=$(az storage account keys list \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --query '[0].value' -o tsv | tr -d '\r')

# Upload config file
echo ""
echo "Uploading config file to file share..."
az storage file upload \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --account-key "$STORAGE_KEY" \
    --share-name "$FILE_SHARE_NAME" \
    --source "$CONFIG_PATH" \
    --path "$CONFIG_FILE_NAME" \
    --no-progress

# Verify upload
echo ""
echo "Verifying upload..."
if az storage file exists \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --account-key "$STORAGE_KEY" \
    --share-name "$FILE_SHARE_NAME" \
    --path "$CONFIG_FILE_NAME" \
    --query exists -o tsv | tr -d '\r' | grep -q "true"; then
    echo "Config file uploaded successfully!"
else
    echo "Error: Failed to verify config file upload"
    exit 1
fi

echo ""
echo "Upload complete!"
