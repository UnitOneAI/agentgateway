#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if config.env exists
if [ ! -f "${SCRIPT_DIR}/config.env" ]; then
    echo "Error: config.env not found!"
    echo "Please copy config.env.template to config.env and fill in your values"
    exit 1
fi

echo "======================================"
echo "AgentGateway Azure Deployment"
echo "======================================"
echo ""

# Parse command line arguments
SKIP_RG=false
SKIP_IMAGE=false
SKIP_CONFIG=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-rg)
            SKIP_RG=true
            shift
            ;;
        --skip-image)
            SKIP_IMAGE=true
            shift
            ;;
        --skip-config)
            SKIP_CONFIG=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-rg       Skip resource group creation"
            echo "  --skip-image    Skip Docker image build and upload"
            echo "  --skip-config   Skip config file upload"
            echo "  --help          Show this help message"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Verify login
echo ""
echo "Current Azure account:"
az account show --query "{Name:name, ID:id, TenantID:tenantId}" -o table

if [ "$SKIP_RG" = false ]; then
    echo "Step 1: Create resource group"
    bash "${SCRIPT_DIR}/create-resource-group.sh"
    echo ""
else
    echo "Step 1: Create resource group (SKIPPED)"
    echo ""
fi

if [ "$SKIP_CONFIG" = false ]; then
    echo "Step 2: Upload Configuration"
    bash "${SCRIPT_DIR}/create-fileshare.sh"
    echo ""

    bash "${SCRIPT_DIR}/upload-config.sh"
    echo ""
else
    echo "Step 2: Upload Configuration (SKIPPED)"
    echo ""
fi

if [ "$SKIP_IMAGE" = false ]; then
    echo "Step 3: Build and Push Docker Image"
    bash "${SCRIPT_DIR}/upload-image.sh" #or docker-build.sh if build locally
    echo ""
else
    echo "Step 3: Build and Push Docker Image (SKIPPED)"
    echo ""
fi

echo "Step 4: Create Azure Container Instance"
bash "${SCRIPT_DIR}/create-aci.sh"
echo ""

echo "======================================"
echo "Deployment Completed Successfully!"
echo "======================================"
