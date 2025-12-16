#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

echo "======================================"
echo "Building and Uploading Docker Image"
echo "======================================"

# Create ACR if it doesn't exist
echo "Checking Azure Container Registry..."
if az acr show --name "$ACR_NAME" --resource-group "$AZURE_RESOURCE_GROUP" &>/dev/null; then
    echo "ACR '$ACR_NAME' already exists"
else
    echo "Creating Azure Container Registry '$ACR_NAME'..."
    az acr create \
        --resource-group "$AZURE_RESOURCE_GROUP" \
        --name "$ACR_NAME" \
        --sku Basic \
        --location "$AZURE_LOCATION" \
        --admin-enabled true
fi

# Get ACR login server
echo ""
echo "Getting ACR login server..."
ACR_LOGIN_SERVER=$(az acr show \
    --name "$ACR_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query loginServer -o tsv | tr -d '\r')

echo "ACR Login Server: $ACR_LOGIN_SERVER"

# Build and push image using ACR build
echo ""
echo "Building and pushing Docker image to ACR..."
echo "This may take several minutes..."

# Navigate to project root (two levels up from scripts/azure-deploy)
PROJECT_ROOT="${SCRIPT_DIR}/../.."

az acr build \
    --registry "$ACR_NAME" \
    --image "${IMAGE_NAME}:${IMAGE_TAG}" \
    --file "${PROJECT_ROOT}/Dockerfile" \
    "${PROJECT_ROOT}"

# Verify image
echo ""
echo "Verifying image..."
if az acr repository show \
    --name "$ACR_NAME" \
    --image "${IMAGE_NAME}:${IMAGE_TAG}" &>/dev/null; then
    echo "Image uploaded successfully!"
    echo "Image: ${ACR_LOGIN_SERVER}/${IMAGE_NAME}:${IMAGE_TAG}"
else
    echo "Error: Failed to verify image upload"
    exit 1
fi

echo ""
echo "Image upload complete!"
