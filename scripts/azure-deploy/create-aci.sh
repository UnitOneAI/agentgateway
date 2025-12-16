#!/bin/bash
set -e

# Load configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/config.env"

# Default OS type if not set
ACI_OS_TYPE="${ACI_OS_TYPE:-Linux}"

echo "======================================"
echo "Creating Azure Container Instance"
echo "======================================"

# Get ACR credentials
echo "Retrieving ACR credentials..."
ACR_LOGIN_SERVER=$(az acr show \
    --name "$ACR_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query loginServer -o tsv | tr -d '\r')

ACR_USERNAME=$(az acr credential show \
    --name "$ACR_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query username -o tsv | tr -d '\r')

ACR_PASSWORD=$(az acr credential show \
    --name "$ACR_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query "passwords[0].value" -o tsv | tr -d '\r')

# Get storage account key
echo "Retrieving storage account key..."
STORAGE_KEY=$(az storage account keys list \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --account-name "$STORAGE_ACCOUNT_NAME" \
    --query '[0].value' -o tsv | tr -d '\r')

# Delete existing ACI if it exists
echo ""
if az container show \
    --name "$ACI_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" &>/dev/null; then
    echo "Deleting existing container instance '$ACI_NAME'..."
    az container delete \
        --name "$ACI_NAME" \
        --resource-group "$AZURE_RESOURCE_GROUP" \
        --yes
    echo "Waiting for deletion to complete..."
    sleep 10
fi

# Create ACI
echo ""
echo "Creating container instance '$ACI_NAME'..."
az container create \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --name "$ACI_NAME" \
    --image "${ACR_LOGIN_SERVER}/${IMAGE_NAME}:${IMAGE_TAG}" \
    --os-type "$ACI_OS_TYPE" \
    --registry-login-server "$ACR_LOGIN_SERVER" \
    --registry-username "$ACR_USERNAME" \
    --registry-password "$ACR_PASSWORD" \
    --ip-address Public \
    --dns-name-label "$ACI_DNS_NAME" \
    --cpu "$ACI_CPU" \
    --memory "$ACI_MEMORY" \
    --ports $EXPOSED_PORTS \
    --azure-file-volume-account-name "$STORAGE_ACCOUNT_NAME" \
    --azure-file-volume-account-key "$STORAGE_KEY" \
    --azure-file-volume-share-name "$FILE_SHARE_NAME" \
    --azure-file-volume-mount-path "$MOUNT_PATH" \
    --command-line "/app/agentgateway -f ${MOUNT_PATH}/${CONFIG_FILE_NAME}" \
    --restart-policy Always

# Get container details
echo ""
echo "Retrieving container details..."
FQDN=$(az container show \
    --name "$ACI_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query ipAddress.fqdn -o tsv | tr -d '\r')

IP=$(az container show \
    --name "$ACI_NAME" \
    --resource-group "$AZURE_RESOURCE_GROUP" \
    --query ipAddress.ip -o tsv | tr -d '\r')

echo ""
echo "======================================"
echo "Deployment Complete!"
echo "======================================"
echo "Container Instance: $ACI_NAME"
echo "FQDN: $FQDN"
echo "IP Address: $IP"
echo "Exposed Ports: $EXPOSED_PORTS"
echo ""
echo "Access URLs:"
for port in $EXPOSED_PORTS; do
    echo "  - http://${FQDN}:${port}"
done
echo ""
echo "View logs with:"
echo "  az container logs --name $ACI_NAME --resource-group $AZURE_RESOURCE_GROUP"
echo ""
echo "Follow logs with:"
echo "  az container attach --name $ACI_NAME --resource-group $AZURE_RESOURCE_GROUP"
