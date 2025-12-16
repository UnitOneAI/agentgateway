# Azure Container Instance Deployment Scripts

This directory contains scripts to deploy AgentGateway to Azure Container Instances (ACI) with configuration stored on Azure File Share.

## Prerequisites

### Azure Permissions Required

User has to be logged in using Azure CLI (az login)

Azure account needs permissions to:
- Create/manage Resource Groups
- Create/manage Storage Accounts and File Shares
- Create/manage Azure Container Registry (ACR)
- Create/manage Azure Container Instances (ACI)

Typically, **Contributor** role on the subscription or resource group is sufficient.

### Required

| Tool | Version | Description |
|------|---------|-------------|
| **Azure CLI** | 2.50+ | Command-line tool for Azure management |
| **Bash** | 4.0+ | Shell for running scripts |
| **Azure Subscription** | - | Active subscription with permissions to create resources |

### Optional

| Tool | Version | Description |
|------|---------|-------------|
| **Docker** | 20.0+ | Only needed if building images locally (not required for ACR build) |

### Installing Azure CLI

**Windows:**
```powershell
winget install Microsoft.AzureCLI
# or download MSI from https://aka.ms/installazurecliwindows
```

**macOS:**
```bash
brew install azure-cli
```

**Linux (Ubuntu/Debian):**
```bash
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
```

**Verify installation:**
```bash
az --version
```

### Bash Shell Setup

**Windows users** - use one of these:
- **WSL2** (recommended): `wsl --install` in PowerShell
- **Git Bash**: Included with [Git for Windows](https://gitforwindows.org/)
- **MSYS2**: https://www.msys2.org/

**macOS/Linux:** Bash is pre-installed.

## Quick Start

1. Copy the configuration template:
   ```bash
   cp config.env.template config.env
   ```

2. Edit `config.env` and fill in your values:
   - Customize resource names, locations, and ports as needed
   - Update `CONFIG_FILE_PATH` to point to your config file

3. Run the complete deployment:
   ```bash
   bash deploy.sh
   ```

## Configuration

Edit `config.env` to customize your deployment:

### Azure Configuration
- `AZURE_RESOURCE_GROUP`: Resource group name
- `AZURE_LOCATION`: Azure region (e.g., eastus, westus2)

### Container Registry
- `ACR_NAME`: Azure Container Registry name (must be globally unique and alphanumerical)
- `IMAGE_NAME`: Docker image name
- `IMAGE_TAG`: Docker image tag

### Storage
- `STORAGE_ACCOUNT_NAME`: Storage account name (must be globally unique)
- `FILE_SHARE_NAME`: File share name for config storage

### Container Instance
- `ACI_NAME`: Container instance name
- `ACI_DNS_NAME`: DNS label for FQDN (must be globally unique in region)
- `ACI_CPU`: CPU cores (e.g., 1.0, 2.0)
- `ACI_MEMORY`: Memory in GB (e.g., 1.5, 2.0)

### Application
- `CONFIG_FILE_PATH`: Path to your config.yaml file
- `EXPOSED_PORTS`: Space-separated list of ports to expose (e.g., "3000 3001 15000")

## Individual Scripts

You can run individual scripts for specific tasks:

### 1. Create  Azure resource group
```bash
bash create-resource-group.sh
```
Creates esource group.

### 2. Create File Share
```bash
bash create-fileshare.sh
```
Creates storage account and file share for configuration.

### 3. Upload Configuration
```bash
bash upload-config.sh
```
Uploads your config.yaml to the Azure file share.

### 4. Build and Upload Docker Image
```bash
bash upload-image.sh
```
Builds the Docker image and pushes it to Azure Container Registry.
Uses `az acr build` which builds in Azure (no local Docker required).

### 5. Create Container Instance
```bash
bash create-aci.sh
```
Creates the Azure Container Instance with:
- Public IP and FQDN
- Multiple exposed ports
- Config file mounted from Azure File Share
- Auto-restart policy

## Main Deployment Script

The `deploy.sh` script runs all steps in sequence:

```bash
bash deploy.sh
```

### Skip Options

You can skip certain steps if they're already completed:

```bash
# Skip resource group creation
bash deploy.sh --skip-rg

# Skip image build (if image already uploaded)
bash deploy.sh --skip-image

# Skip config upload (if config already uploaded)
bash deploy.sh --skip-config

# Combine multiple skip options
bash deploy.sh --skip-rg --skip-config
```

## Accessing Your Deployment

After deployment completes, you'll see output like:

```
FQDN: agentgateway.eastus2.azurecontainer.io
IP Address: 20.62.xxx.xxx
Exposed Ports: 3000 3001 15000

Access URLs:
  - http://agentgateway.eastus2.azurecontainer.io:3000
  - http://agentgateway.eastus2.azurecontainer.io:3001
  - http://agentgateway.eastus2.azurecontainer.io:15000
```

## Monitoring and Logs

View container logs:
```bash
az container logs --name <ACI_NAME> --resource-group <RESOURCE_GROUP>
```

Follow logs in real-time:
```bash
az container attach --name <ACI_NAME> --resource-group <RESOURCE_GROUP>
```

Check container status:
```bash
az container show --name <ACI_NAME> --resource-group <RESOURCE_GROUP>
```

## Updating Configuration

To update the configuration file:

1. Edit your local config.yaml
2. Run the upload script:
   ```bash
   bash upload-config.sh
   ```
3. Restart the container if needed (usually config hot-reloaded):
   ```bash
   az container restart --name <ACI_NAME> --resource-group <RESOURCE_GROUP>
   ```

## Updating the Application

To deploy a new version:

1. Build and upload new image:
   ```bash
   bash upload-image.sh
   ```

2. Recreate the container instance:
   ```bash
   bash create-aci.sh
   ```

Or run the full deployment with skip options:
```bash
bash deploy.sh --skip-rg --skip-config
```

## Cleanup

To delete all resources:

```bash
az group delete --name <RESOURCE_GROUP> --yes --no-wait
```

## Troubleshooting

### Container fails to start
- Check logs: `az container logs --name <ACI_NAME> --resource-group <RESOURCE_GROUP>`
- Verify config file is valid
- Ensure ports in config.yaml match EXPOSED_PORTS

### Can't access the service
- Verify FQDN is resolving: `nslookup <FQDN>`
- Check security group rules (ACI has no NSG by default, all ports are open)
- Verify the application is listening on 0.0.0.0, not 127.0.0.1

### Storage key errors on Windows
- The scripts use `tr -d '\r'` to strip carriage returns
- If issues persist, use WSL or Git Bash instead of Command Prompt

### ACR access denied
- Verify ACR admin user is enabled
- Check credentials: `az acr credential show --name <ACR_NAME>`

## Notes

- All Azure CLI outputs are stripped of carriage returns (`\r`) for Windows compatibility
- The scripts use `set -e` to exit on any error
- ACR build is used for image building (no local Docker required)
- File share quota is set to 1GB (can be adjusted in create-fileshare.sh)
- Container restart policy is set to "Always"
