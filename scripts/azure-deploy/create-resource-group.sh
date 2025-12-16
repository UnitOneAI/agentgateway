#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

echo "======================================"
echo "Azure create resource group"
echo "======================================"

# Create resource group if it doesn't exist
echo ""
echo "Checking resource group..."
if az group show --name "$AZURE_RESOURCE_GROUP" &>/dev/null; then
    echo "Resource group '$AZURE_RESOURCE_GROUP' already exists"
else
    echo "Creating resource group '$AZURE_RESOURCE_GROUP' in '$AZURE_LOCATION'..."
    az group create --name "$AZURE_RESOURCE_GROUP" --location "$AZURE_LOCATION"
fi

echo ""
echo "Login complete!"
