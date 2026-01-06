# Deployment Guide

This document explains the deployment process for UnitOne AgentGateway.

## Overview

The deployment is fully automated using:
- **Terraform** for infrastructure-as-code (managed in separate `terraform` repository)
- **Azure Container Registry (ACR)** for Docker image storage
- **ACR Tasks** for automated builds on git push
- **Azure Container Apps** for hosting

## Deployment Architecture

### Infrastructure (Terraform)

All infrastructure is managed via Terraform in the **separate `terraform` repository**:
- Location: `terraform/modules/azure/agentgateway/`
- Environment configs: `terraform/environments/dev/agentgateway/`

**Resources managed by Terraform:**
- Azure Container Registry (ACR)
- Log Analytics Workspace
- Application Insights
- Key Vault (for OAuth secrets)
- Container Apps Environment
- Container App with:
  - Managed identity
  - Ingress configuration
  - Auto-scaling rules
- ACR Task (for automated builds)

### CI/CD Pipeline

**Automated Build & Deploy Flow:**

1. **Developer pushes to `main`** (or merges PR)
2. **ACR Task automatically triggers** (configured via Terraform)
3. **ACR builds Docker image** using `Dockerfile.acr`
4. **Image tagged and pushed** to ACR:
   - `unitone-agentgateway:latest`
   - `unitone-agentgateway:{{.Run.ID}}`
5. **Container App auto-refreshes** (if `enable_auto_deployment = true` in Terraform)
6. **New revision deployed** automatically

**No GitHub Actions needed for Azure deployment!** The ACR Task handles everything.

### GitHub Workflows

This repository contains GitHub workflows for **CI testing only**:

- **`.github/workflows/pull_request.yml`**: Runs on PRs and pushes to `main`
  - Builds on multiple platforms (Linux x86/ARM, macOS, Windows)
  - Builds UI (`npm run build`)
  - Runs tests (`make test`)
  - Runs validation (`make validate`)
  - Runs linting
  - **Does NOT deploy to Azure**

- **`.github/workflows/release.yml`**: Publishes releases
  - Creates semantic version tags (v1.0.0)
  - Builds and pushes to **GitHub Container Registry** (`ghcr.io`)
  - **Does NOT deploy to Azure**

- **`.github/workflows/e2e-tests.yml`**: E2E testing

## Configuration

### Terraform Variables

Configure deployment behavior in Terraform:

```hcl
# terraform/environments/dev/agentgateway/terraform.tfvars
enable_auto_deployment = true  # Auto-deploy on successful build
image_tag             = "latest"
github_pat            = "ghp_xxx..."  # Required for ACR Task

# OAuth secrets
microsoft_client_id     = "..."
microsoft_client_secret = "..."
github_client_id        = "..."
github_client_secret    = "..."
google_client_id        = "..."
google_client_secret    = "..."
```

### ACR Task Configuration

The ACR Task is configured in Terraform (`modules/azure/agentgateway/ci_cd.tf`):

**Triggers:**
- Git commits to `main` branch
- Base image updates (security patches)

**Build configuration:**
- Repository: `https://github.com/UnitOneAI/unitone-agentgateway.git`
- Dockerfile: `Dockerfile.acr`
- CPU: 2 cores

## Environments

### Dev Environment
- **Resource Group**: `mcp-gateway-dev-rg`
- **ACR**: `unitoneagwdevacr`
- **Container App**: `unitone-agw-dev-app`
- **URL**: https://unitone-agw-dev-app.azurewebsites.net
- **Auto-deploy**: Enabled on push to `main`
- **Scaling**: 1-3 replicas

### Production Environment
- **Resource Group**: `mcp-gateway-prod-rg` (if configured)
- **ACR**: `unitoneagwprodacr`
- **Container App**: `unitone-agw-prod-app`
- **Auto-deploy**: Typically manual/controlled
- **Scaling**: 2-10 replicas

## Manual Operations

### Build and Push Image Manually

If you need to build without triggering via git push:

```bash
cd /Users/surindersingh/source_code/agentgateway

# Build and push to ACR
az acr build --registry unitoneagwdevacr \
  --image unitone-agentgateway:latest \
  --image unitone-agentgateway:my-feature \
  --file Dockerfile.acr \
  --platform linux/amd64 \
  .
```

### Deploy Specific Image Tag

```bash
# Update Container App with specific tag
az containerapp update \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --image unitoneagwdevacr.azurecr.io/unitone-agentgateway:my-feature
```

### Deploy Infrastructure Changes

```bash
cd /path/to/terraform/repo
cd environments/dev/agentgateway

# Review changes
terraform plan

# Apply changes
terraform apply
```

## Monitoring Deployments

### Check ACR Build Status

```bash
# List recent builds
az acr task list-runs --registry unitoneagwdevacr -o table

# View build logs
az acr task logs --registry unitoneagwdevacr --run-id <run-id>
```

### Check Container App Status

```bash
# List revisions
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "[].{Name:name, Active:properties.active, Created:properties.createdTime, TrafficWeight:properties.trafficWeight}" \
  -o table

# View logs
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --follow

# Get app URL
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.configuration.ingress.fqdn" \
  -o tsv
```

### Azure Portal

- **Container App**: Monitor revisions, logs, metrics
- **Application Insights**: View telemetry, errors, performance
- **Log Analytics**: Query logs across all components
- **ACR**: View build history, task runs, images

## Rollback Procedures

### Using Azure CLI

```bash
# List all revisions
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "[].{Name:name, Active:properties.active, Created:properties.createdTime, Image:properties.template.containers[0].image}" \
  -o table

# Activate previous revision
az containerapp revision activate \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --revision <previous-revision-name>

# Deactivate bad revision
az containerapp revision deactivate \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --revision <bad-revision-name>
```

### Using Azure Portal

1. Navigate to Container App → Revisions
2. Select previous healthy revision
3. Click "Activate" and set traffic to 100%

### Rebuild Previous Version

```bash
# Deploy with specific image tag from ACR
az containerapp update \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --image unitoneagwdevacr.azurecr.io/unitone-agentgateway:<previous-tag>
```

## Troubleshooting

### Build Fails

Check ACR Task logs:
```bash
az acr task logs --registry unitoneagwdevacr --name agentgateway-build-task
```

Common issues:
- GitHub PAT expired or invalid
- Dockerfile.acr syntax errors
- Build dependencies missing

### Container App Not Starting

```bash
# Check logs
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --tail 100

# Check current status
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "{ProvisioningState:properties.provisioningState, RunningState:properties.runningStatus}" \
  -o table
```

Common issues:
- Image tag doesn't exist in ACR
- Environment variables missing
- OAuth secrets not configured
- Port 8080 not exposed

### OAuth Not Working

Verify secrets in Key Vault:
```bash
az keyvault secret list --vault-name unitone-agw-dev-kv -o table
az keyvault secret show --vault-name unitone-agw-dev-kv --name github-client-id
```

Check redirect URIs are configured in OAuth providers (GitHub, Microsoft, Google).

### UI Returns 404

- Verify UI was built during Docker image creation
- Check Dockerfile.acr includes UI build steps
- Ensure static files are served correctly

## Security Best Practices

1. **Secrets Management**:
   - Never commit secrets to Git
   - Use Azure Key Vault for OAuth secrets
   - Use Terraform `sensitive = true` for secret variables
   - Rotate GitHub PAT regularly

2. **Image Security**:
   - Scan images for vulnerabilities
   - Keep base images updated (ACR Task auto-rebuilds)
   - Use minimal base images
   - Review dependencies regularly

3. **Access Control**:
   - Use Managed Identity for Azure resource access
   - Restrict ACR access with RBAC
   - Limit service principal permissions
   - Enable OAuth for UI access

4. **Network Security**:
   - Use HTTPS for all endpoints
   - Configure CORS policies appropriately
   - Consider private endpoints for production

## Development Workflow

### Making Code Changes

1. Create feature branch: `git checkout -b feature/my-change`
2. Make changes and test locally
3. Commit and push: `git push origin feature/my-change`
4. Create PR to `main`
5. PR triggers CI tests (`.github/workflows/pull_request.yml`)
6. After approval and merge to `main`:
   - ACR Task automatically builds new image
   - Container App auto-deploys (if enabled)
7. Verify deployment in Azure

### Testing Before Deployment

```bash
# Build locally
make docker

# Run locally
docker run -p 8080:8080 agentgateway:latest

# Run tests
make test
make validate
```

## Reference Documentation

- **Terraform Configuration**: See `terraform` repository
  - Module: `modules/azure/agentgateway/`
  - CI/CD: `modules/azure/agentgateway/ci_cd.tf`
  - Environment: `environments/dev/agentgateway/`
- **ACR Tasks**: https://docs.microsoft.com/azure/container-registry/container-registry-tasks-overview
- **Container Apps**: https://docs.microsoft.com/azure/container-apps/
- **Terraform Azure Provider**: https://registry.terraform.io/providers/hashicorp/azurerm/latest/docs

## Next Steps

For new deployments:
1. Set up Terraform configuration (see `terraform` repository)
2. Configure GitHub PAT for ACR Task
3. Set OAuth secrets in terraform.tfvars
4. Run `terraform apply`
5. Push code to `main` to trigger first build
6. Monitor deployment in Azure Portal
