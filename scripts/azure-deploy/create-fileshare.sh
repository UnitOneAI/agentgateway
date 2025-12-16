#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

echo "======================================"
echo "Creating Azure File Share"
echo "======================================"

# Create storage account if it doesn't exist
echo "Checking storage account..."
if az storage account show --name "$STORAGE_ACCOUNT_NAME" --resource-group "$AZURE_RESOURCE_GROUP" &>/dev/null; then
    echo "Storage account '$STORAGE_ACCOUNT_NAME' already exists"
else
    echo "Creating storage account '$STORAGE_ACCOUNT_NAME'..."
    az storage account create \
        --name "$STORAGE_ACCOUNT_NAME" \
        --resource-group "$AZURE_RESOURCE_GROUP" \
        --location "$AZURE_LOCATION" \
        --sku Standard_LRS \
        --kind StorageV2
fi

# Get storage account key
echo ""
echo "Retrieving storage account key..."
STORAGE_KEY=$(az storage account keys list \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --query '[0].value' -o tsv | tr -d '\r')

# Create file share if it doesn't exist
echo ""
echo "Checking file share..."
if az storage share show \
    --name "$FILE_SHARE_NAME" \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --account-key "$STORAGE_KEY" &>/dev/null; then
    echo "File share '$FILE_SHARE_NAME' already exists"
else
    echo "Creating file share '$FILE_SHARE_NAME'..."
    az storage share create \
        --name "$FILE_SHARE_NAME" \
        --account-name "$STORAGE_ACCOUNT_NAME" \
        --account-key "$STORAGE_KEY" \
        --quota 1
fi

echo ""
echo "File share setup complete!"
echo "Storage Account: $STORAGE_ACCOUNT_NAME"
echo "File Share: $FILE_SHARE_NAME"
