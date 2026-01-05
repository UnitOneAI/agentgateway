#!/bin/bash
# Generic script to wait for ACR build completion before deploying
# Usage: ./wait_for_build_and_deploy.sh <run-id> <image-tag> [resource-group] [app-name]

set -e

# Configuration
RUN_ID="${1:?Error: RUN_ID required as first argument}"
IMAGE_TAG="${2:?Error: IMAGE_TAG required as second argument}"
RESOURCE_GROUP="${3:-mcp-gateway-dev-rg}"
APP_NAME="${4:-unitone-agentgateway}"
REGISTRY="agwimages"

echo "========================================="
echo "Wait-for-Build-and-Deploy Script"
echo "========================================="
echo "Run ID: $RUN_ID"
echo "Image Tag: $IMAGE_TAG"
echo "Resource Group: $RESOURCE_GROUP"
echo "App Name: $APP_NAME"
echo "Registry: $REGISTRY"
echo "========================================="

# Step 1: Wait for build to complete
echo ""
echo "[Step 1/3] Waiting for build $RUN_ID to complete..."

while true; do
  STATUS=$(az acr task logs --registry $REGISTRY --run-id $RUN_ID 2>&1 | grep "Run Status" | tail -1 || echo "")

  if [ -z "$STATUS" ]; then
    echo "⏳ Build status not available yet, waiting..."
    sleep 15
    continue
  fi

  echo "Current status: $STATUS"

  if echo "$STATUS" | grep -q "Succeeded"; then
    echo "✓ Build $RUN_ID completed successfully!"
    break
  elif echo "$STATUS" | grep -q "Failed"; then
    echo "✗ Build $RUN_ID failed!"
    echo ""
    echo "Build logs:"
    az acr task logs --registry $REGISTRY --run-id $RUN_ID 2>&1 | tail -50
    exit 1
  fi

  sleep 15
done

# Step 2: Deploy the image
echo ""
echo "[Step 2/3] Deploying image to Azure Container App..."

az containerapp update \
  --name $APP_NAME \
  --resource-group $RESOURCE_GROUP \
  --image $REGISTRY.azurecr.io/$APP_NAME:$IMAGE_TAG

echo "✓ Deployment triggered successfully!"

# Step 3: Wait for deployment stabilization
echo ""
echo "[Step 3/3] Waiting for deployment to stabilize (30 seconds)..."
sleep 30

echo ""
echo "✓ Deployment complete!"
echo ""
echo "Active revision:"
az containerapp revision list \
  --name $APP_NAME \
  --resource-group $RESOURCE_GROUP \
  --query "[?properties.active].{Name:name, Image:properties.template.containers[0].image, Traffic:properties.trafficWeight}" \
  -o table

echo ""
echo "========================================="
echo "All steps completed successfully!"
echo "========================================="
