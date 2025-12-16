#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

# Navigate to project root
PROJECT_ROOT="${SCRIPT_DIR}/../.."

echo "======================================"
echo "Build & Push Docker Image (Local)"
echo "======================================"

# Parse command line arguments
PLATFORM="${PLATFORM:-linux/amd64}"
BUILDER="${BUILDER:-base}"
PROFILE="${PROFILE:-release}"

# Check prerequisites
command -v docker &>/dev/null || { echo "Error: Docker is not installed"; exit 1; }
command -v az &>/dev/null || { echo "Error: Azure CLI is not installed"; exit 1; }

# Create ACR if it doesn't exist
echo "Checking Azure Container Registry..."
if az acr show --name "$ACR_NAME" --resource-group "$AZURE_RESOURCE_GROUP" &>/dev/null; then
    echo "ACR '$ACR_NAME' exists"
else
    echo "Creating ACR '$ACR_NAME'..."
    az acr create \
        --resource-group "$AZURE_RESOURCE_GROUP" \
        --name "$ACR_NAME" \
        --sku Basic \
        --location "$AZURE_LOCATION" \
        --admin-enabled true
fi

# Get ACR login server
ACR_LOGIN_SERVER=$(az acr show \
    --name "$ACR_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query loginServer -o tsv | tr -d '\r')

echo "ACR Login Server: $ACR_LOGIN_SERVER"

# Login to ACR
echo ""
echo "Logging in to ACR..."
az acr login --name "$ACR_NAME"

# Build image
FULL_IMAGE_NAME="${ACR_LOGIN_SERVER}/${IMAGE_NAME}:${IMAGE_TAG}"

echo ""
echo "Building Docker image..."
echo "  Platform: $PLATFORM"
echo "  Builder:  $BUILDER"
echo "  Profile:  $PROFILE"
echo "  Image:    $FULL_IMAGE_NAME"
echo ""

docker build \
    --platform "$PLATFORM" \
    --build-arg BUILDER="$BUILDER" \
    --build-arg PROFILE="$PROFILE" \
    -t "$FULL_IMAGE_NAME" \
    -f "${PROJECT_ROOT}/Dockerfile" \
    "${PROJECT_ROOT}"

# Push image
echo ""
echo "Pushing image to ACR..."
docker push "$FULL_IMAGE_NAME"

# Verify
echo ""
if az acr repository show --name "$ACR_NAME" --image "${IMAGE_NAME}:${IMAGE_TAG}" &>/dev/null; then
    echo "Image uploaded successfully!"
    echo "Image: $FULL_IMAGE_NAME"
else
    echo "Error: Failed to verify image upload"
    exit 1
fi
