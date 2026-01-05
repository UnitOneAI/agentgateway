# Deployment Automation Guide

This document explains the deployment automation for UnitOne AgentGateway.

## Overview

The deployment process is fully automated using:
- **Bicep** for infrastructure-as-code
- **GitHub Actions** for CI/CD
- **Azure Container Registry (ACR)** for Docker image storage
- **Azure Container Apps** for hosting

## Deployment Workflows

### 1. CI/CD Pipeline (`.github/workflows/pull_request.yml`)

**Triggers**: Push to `main`, Pull Requests to `main`

**What it does**:
- Builds on multiple platforms (Linux x86/ARM, macOS, Windows)
- Builds UI (`npm run build`)
- Runs all tests (`make test`)
- Runs validation (`make validate`)
- Runs linting

**Does NOT**: Deploy to Azure

---

### 2. Azure Deployment (`.github/workflows/azure-deploy.yml`) ⭐ NEW

**Triggers**:
- **Push to `main`**: Automatically deploys to **dev** environment
- **Release published**: Automatically deploys to **prod** environment
- **Manual dispatch**: Deploy to any environment with custom tag

**What it does**:
1. Determines environment and image tag based on trigger
2. Builds Docker image using `az acr build`
3. Multi-tags images:
   - Main tag (commit SHA for dev, semantic version for prod)
   - Timestamp tag (YYYYMMDD-HHMMSS)
   - `latest` tag
4. Deploys infrastructure using Bicep
5. Verifies deployment health
6. Outputs UI and MCP endpoint URLs

**Image Tagging Strategy**:
- **Dev builds** (push to main): `<short-commit-sha>`, `<timestamp>`, `latest`
- **Prod releases** (published release): `<semantic-version>`, `<timestamp>`, `latest`
- **Manual dispatch**: Custom tag + `<timestamp>`, `latest`

---

### 3. Release Workflow (`.github/workflows/release.yml`)

**Triggers**: Manual dispatch or release creation

**What it does**:
- Creates semantic version tags (v1.0.0, v1.0, v1)
- Builds and pushes to GitHub Container Registry (`ghcr.io`)

**Note**: This workflow is separate from Azure deployment. Azure uses ACR (`agwimages.azurecr.io`).

---

## Infrastructure-as-Code (Bicep)

### Files
- **`deploy/bicep/main.bicep`**: Main infrastructure template
- **`deploy/bicep/parameters-dev.json`**: Dev environment parameters
- **`deploy/bicep/parameters-staging.json`**: Staging environment parameters (if exists)
- **`deploy/bicep/parameters-prod.json`**: Prod environment parameters (if exists)

### Resources Defined
- Azure Container Registry (ACR)
- Log Analytics Workspace
- Application Insights
- Key Vault (for OAuth secrets)
- Container Apps Environment
- Container App with:
  - Managed identity
  - Ingress configuration
  - CORS policy
  - OAuth environment variables
  - Auto-scaling rules

---

## GitHub Secrets Required

To enable automated deployments, configure these secrets in GitHub repository settings:

### `AZURE_CREDENTIALS`
Azure Service Principal credentials in JSON format:

```json
{
  "clientId": "<client-id>",
  "clientSecret": "<client-secret>",
  "subscriptionId": "<subscription-id>",
  "tenantId": "<tenant-id>"
}
```

**How to create**:
```bash
az ad sp create-for-rbac \
  --name "github-actions-agentgateway" \
  --role contributor \
  --scopes /subscriptions/<SUBSCRIPTION_ID>/resourceGroups/unitone-agw-dev-rg \
  --sdk-auth
```

### Environment-Specific Secrets

For each environment (dev, staging, prod), configure:
- OAuth client IDs and secrets (stored in Azure Key Vault)
- Any environment-specific configuration

---

## Environment Configuration

### Dev Environment
- **Resource Group**: `unitone-agw-dev-rg`
- **ACR**: `unitoneagwdevacr`
- **Container App**: `unitone-agw-dev-app`
- **Auto-deploy**: On push to `main`
- **Scaling**: 1-3 replicas

### Staging Environment (if configured)
- **Resource Group**: `unitone-agw-staging-rg`
- **ACR**: `unitoneagwstagingacr`
- **Container App**: `unitone-agw-staging-app`
- **Auto-deploy**: Manual dispatch
- **Scaling**: 1-5 replicas

### Prod Environment
- **Resource Group**: `unitone-agw-prod-rg`
- **ACR**: `unitoneagwprodacr`
- **Container App**: `unitone-agw-prod-app`
- **Auto-deploy**: On release published
- **Scaling**: 2-10 replicas

---

## Manual Deployment

If you need to deploy manually, use the deployment script:

```bash
cd deploy

# Deploy to dev (build + deploy)
./deploy.sh --environment dev --build --tag latest

# Deploy to staging
./deploy.sh --environment staging --build --tag v1.0.0

# Deploy to prod
./deploy.sh --environment prod --build --tag v1.0.0

# Deploy without building (use existing image)
./deploy.sh --environment dev --tag existing-tag
```

**Script Options**:
- `-e, --environment ENV`: Environment to deploy (dev, staging, prod) [default: dev]
- `-b, --build`: Build and push Docker image before deploying
- `-t, --tag TAG`: Docker image tag [default: latest]
- `-s, --subscription ID`: Azure subscription ID
- `-h, --help`: Show help message

---

## Deployment Process Flow

### Automated Dev Deployment (Push to Main)

1. **Trigger**: Developer pushes code to `main` branch
2. **CI Tests**: `pull_request.yml` runs tests, validation, linting
3. **Build**: `azure-deploy.yml` builds Docker image with commit SHA tag
4. **Push to ACR**: Image pushed to `unitoneagwdevacr.azurecr.io`
5. **Deploy**: Bicep template updates Container App with new image
6. **Verify**: Workflow checks Container App health status
7. **Notify**: Workflow outputs deployment URLs in GitHub Actions UI

### Automated Prod Deployment (Release Published)

1. **Trigger**: Maintainer publishes a release (e.g., v1.2.0)
2. **Build**: `azure-deploy.yml` builds Docker image with semantic version tag
3. **Push to ACR**: Image pushed to `unitoneagwprodacr.azurecr.io`
4. **Deploy**: Bicep template updates Container App with new image
5. **Verify**: Workflow checks Container App health status
6. **Notify**: Workflow outputs deployment URLs

### Manual Deployment (Workflow Dispatch)

1. **Trigger**: Navigate to Actions → Azure Deployment → Run workflow
2. **Select**: Choose environment (dev/staging/prod) and optional custom tag
3. **Build**: Builds Docker image with specified or commit SHA tag
4. **Deploy**: Same as above
5. **Verify**: Same as above

---

## Image Tag History

All image tags created during this development session:

### Development Tags (Manual)
- `security-guards-tests-pass` - Security guards implementation with all tests passing
- `config-fixed` - Configuration fixes
- `oauth-v1`, `oauth-v2-fixed` - OAuth integration fixes
- `user-menu` - User menu feature
- `ui-enabled`, `admin-api-fix` - UI and admin API fixes
- `verify-config`, `working`, `fixed-final` - Various development iterations

**Note**: Going forward, these manual tags will be replaced by automated tags based on commit SHA (dev) and semantic versioning (prod).

### Automated Tags (Future)
- **Dev**: `<commit-sha>` (e.g., `a1b2c3d`), `YYYYMMDD-HHMMSS`, `latest`
- **Prod**: `<version>` (e.g., `1.2.0`), `YYYYMMDD-HHMMSS`, `latest`

---

## Rollback Procedure

### Using Azure Portal
1. Navigate to Container App → Revisions
2. Select previous healthy revision
3. Click "Activate" and set traffic to 100%

### Using Azure CLI
```bash
# List all revisions
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --query "[].{Name:name, Active:properties.active, Created:properties.createdTime, Image:properties.template.containers[0].image}" \
  -o table

# Activate specific revision
az containerapp revision activate \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --revision <revision-name>

# Deactivate current revision
az containerapp revision deactivate \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --revision <bad-revision-name>
```

### Using GitHub Actions (Manual Dispatch)
1. Go to Actions → Azure Deployment → Run workflow
2. Select the environment
3. Enter the previous working tag (check ACR for available tags)
4. Run workflow

---

## Monitoring Deployments

### GitHub Actions
- View workflow runs at: https://github.com/<org>/<repo>/actions
- Each run shows:
  - Build logs
  - Deployment logs
  - Health check results
  - Deployment URLs

### Azure Portal
- **Container App**: Monitor revisions, logs, metrics
- **Application Insights**: View telemetry, errors, performance
- **Log Analytics**: Query logs across all components

### Azure CLI
```bash
# Follow logs
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --follow

# Get Container App URL
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --query "properties.configuration.ingress.fqdn" \
  -o tsv

# Check health status
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --query "{ProvisioningState:properties.provisioningState, RunningState:properties.runningStatus}" \
  -o table
```

---

## Troubleshooting

### Deployment Fails with "Not Authorized"
- Check `AZURE_CREDENTIALS` secret is properly configured
- Verify Service Principal has Contributor role on resource group
- Ensure subscription ID is correct

### Image Build Fails
- Check Dockerfile.acr exists and is valid
- Verify ACR exists and has sufficient permissions
- Check build logs in GitHub Actions workflow run

### Container App Not Starting
- Check container logs: `az containerapp logs show`
- Verify environment variables are set correctly
- Check if OAuth secrets are properly configured in Key Vault
- Verify image was successfully pushed to ACR

### UI Returns 404
- Verify UI was built during Docker image creation
- Check Dockerfile.acr includes UI build steps
- Ensure ingress is configured correctly in Bicep

### OAuth Not Working
- Verify OAuth client IDs and secrets in Key Vault
- Check redirect URIs are configured in OAuth provider
- Verify environment variables are passed to container

---

## Security Best Practices

1. **Secrets Management**:
   - Never commit secrets to Git
   - Use Azure Key Vault for OAuth secrets
   - Use GitHub Secrets for Azure credentials

2. **Image Security**:
   - Scan images for vulnerabilities before deployment
   - Use minimal base images
   - Keep dependencies up to date

3. **Access Control**:
   - Use Managed Identity for Azure resource access
   - Restrict ACR access with role-based access control
   - Limit Service Principal permissions to specific resource groups

4. **Network Security**:
   - Use HTTPS for all endpoints
   - Configure CORS policies appropriately
   - Enable OAuth authentication for UI

---

## Next Steps

1. **Set up GitHub Secrets**: Configure `AZURE_CREDENTIALS` secret
2. **Test Workflow**: Make a small change and push to `main` to trigger dev deployment
3. **Monitor**: Watch the GitHub Actions workflow run and verify deployment
4. **Create Release**: When ready, create a release to deploy to production
5. **Document**: Update this file with any environment-specific configuration
