# UnitOne AgentGateway - Azure Deployment Guide

Complete Infrastructure-as-Code deployment for AgentGateway with OAuth support.

> **Note**: Infrastructure is now managed via **Terraform** in a separate `terraform` repository. This guide covers the deployment from the application perspective.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Azure Container App                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  AgentGateway (Port 8080)                                  │ │
│  │  ┌──────────────┬──────────────┬──────────────────────┐   │ │
│  │  │ /ui          │ /mcp/*       │ /.well-known/*       │   │ │
│  │  │ (Admin UI)   │ (MCP OAuth)  │ (OAuth metadata)     │   │ │
│  │  └──────────────┴──────────────┴──────────────────────┘   │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                            │
          ┌─────────────────┴─────────────────┐
          │                                    │
   ┌──────▼──────┐                    ┌───────▼────────┐
   │  Key Vault  │                    │  App Insights  │
   │  (Secrets)  │                    │  (Monitoring)  │
   └─────────────┘                    └────────────────┘
```

## Features

- **OAuth Authentication**: GitHub, Microsoft (Azure AD), Google
- **Key Vault Integration**: Secure secret management with Managed Identity
- **MCP-Native OAuth**: Built-in support for MCP Authorization spec
- **Infrastructure as Code**: Full Terraform templates for reproducible deployments
- **Multi-Environment**: Dev, Staging, Production configurations
- **Monitoring**: Application Insights integration
- **Auto-Scaling**: HTTP-based scaling rules
- **Automated CI/CD**: ACR Tasks for automatic builds on git push

## Prerequisites

1. **Azure CLI** installed and logged in:
   ```bash
   az login
   az account set --subscription <YOUR_SUBSCRIPTION_ID>
   ```

2. **OAuth App Registrations** (for each provider you want to use):
   - **GitHub**: https://github.com/settings/applications/new
   - **Microsoft/Azure AD**: https://portal.azure.com → Azure AD → App registrations
   - **Google**: https://console.cloud.google.com/apis/credentials

3. **Docker** (optional, for local testing):
   ```bash
   docker --version
   ```

4. **Terraform** (for infrastructure changes):
   - See separate `terraform` repository

## Deployment Overview

### Infrastructure Management (Terraform)

All infrastructure is managed via **Terraform** in a separate repository:
- Location: `terraform/modules/azure/agentgateway/`
- Environment configs: `terraform/environments/dev/agentgateway/`

**Resources managed by Terraform:**
- Azure Container Registry (ACR)
- Log Analytics Workspace
- Application Insights
- Key Vault (for OAuth secrets)
- Container Apps Environment
- Container App with auto-scaling
- ACR Task (for automated builds)

### Automated Deployment Flow

1. **Developer pushes to `main`** (or merges PR)
2. **ACR Task automatically triggers** (configured via Terraform)
3. **Docker image built** using `Dockerfile.acr`
4. **Image pushed to ACR** with tags: `latest` and `{{.Run.ID}}`
5. **Container App auto-refreshes** (if enabled in Terraform)

**No manual deployment needed!** Infrastructure handles everything.

## Quick Start

### 1. Setup OAuth Secrets (First Time Only)

Configure OAuth secrets in Terraform variables file:

```bash
cd /path/to/terraform/repo
cd environments/dev/agentgateway

# Create terraform.tfvars
cat > terraform.tfvars <<EOF
# OAuth Configuration
microsoft_client_id     = "YOUR_MICROSOFT_CLIENT_ID"
microsoft_client_secret = "YOUR_MICROSOFT_CLIENT_SECRET"
github_client_id        = "YOUR_GITHUB_CLIENT_ID"
github_client_secret    = "YOUR_GITHUB_CLIENT_SECRET"
google_client_id        = "YOUR_GOOGLE_CLIENT_ID"
google_client_secret    = "YOUR_GOOGLE_CLIENT_SECRET"

# GitHub PAT for ACR Task
github_pat = "ghp_xxxxx..."

# Deployment settings
enable_auto_deployment = true
image_tag             = "latest"
EOF

# Keep secrets safe
chmod 600 terraform.tfvars
```

### 2. Deploy Infrastructure (First Time Only)

```bash
cd /path/to/terraform/repo
cd environments/dev/agentgateway

# Initialize Terraform
terraform init

# Review planned changes
terraform plan

# Apply infrastructure
terraform apply
```

### 3. Deploy Application Code

**Automated (Recommended)**:
```bash
# Just push to main!
git push origin main

# ACR Task builds and deploys automatically
```

**Manual (for testing)**:
```bash
# Build and push to ACR manually
cd /path/to/agentgateway/repo
az acr build --registry unitoneagwdevacr \
  --image unitone-agentgateway:latest \
  --image unitone-agentgateway:my-feature \
  --file Dockerfile.acr \
  --platform linux/amd64 \
  .

# Deploy specific tag (optional)
az containerapp update \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --image unitoneagwdevacr.azurecr.io/unitone-agentgateway:my-feature
```

### 4. Access Your Deployment

Get deployment URLs:
```bash
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.configuration.ingress.fqdn" \
  -o tsv
```

Output:
```
https://unitone-agw-dev-app.....azurecontainerapps.io
```

Access points:
- **UI**: `https://<fqdn>/ui`
- **MCP Endpoint**: `https://<fqdn>/mcp`
- **OAuth Metadata**: `https://<fqdn>/.well-known/oauth-protected-resource/mcp/<provider>`

## OAuth Configuration

### Supported OAuth Providers

#### 1. GitHub OAuth

**Use Case**: GitHub Actions, GitHub Apps

**Endpoints**:
- MCP: `/mcp/github`
- Metadata: `/.well-known/oauth-protected-resource/mcp/github`

**Required Scopes**: `read:all`, `write:all`

**Setup**:
1. Create OAuth App at https://github.com/settings/applications/new
2. Set Authorization callback URL: `https://<your-fqdn>/oauth/callback`
3. Add Client ID and Secret to Terraform variables

#### 2. Microsoft Azure AD

**Use Case**: Enterprise SSO, Microsoft 365 integration

**Endpoints**:
- MCP: `/mcp/microsoft`
- Metadata: `/.well-known/oauth-protected-resource/mcp/microsoft`

**Required Scopes**: `api://unitone-agentgateway/read`, `api://unitone-agentgateway/write`

**Setup**:
1. Register app in Azure AD
2. Configure API permissions
3. Add Client ID and Secret to Terraform variables

#### 3. Google OAuth

**Use Case**: Google Workspace, Gmail integration

**Endpoints**:
- MCP: `/mcp/google`
- Metadata: `/.well-known/oauth-protected-resource/mcp/google`

**Required Scopes**: `openid`, `profile`, `email`

**Setup**:
1. Create OAuth 2.0 Client in Google Cloud Console
2. Add authorized redirect URI
3. Add Client ID and Secret to Terraform variables

### Testing OAuth Endpoints

```bash
# Test without token (should return 401)
curl -i https://your-app.azurecontainerapps.io/mcp/github

# Expected response:
# HTTP/1.1 401 Unauthorized
# WWW-Authenticate: Bearer resource_metadata="https://your-app.azurecontainerapps.io/.well-known/oauth-protected-resource/mcp/github"

# Test with valid token
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
     https://your-app.azurecontainerapps.io/mcp/github
```

## Monitoring and Operations

### View Container Logs

```bash
# Follow logs in real-time
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --follow

# View last 100 lines
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --tail 100
```

### Check Build Status

```bash
# List recent ACR builds
az acr task list-runs --registry unitoneagwdevacr -o table

# View specific build logs
az acr task logs --registry unitoneagwdevacr --run-id <run-id>

# Follow live build
az acr task logs --registry unitoneagwdevacr --name agentgateway-build-task --follow
```

### Check Deployment Status

```bash
# List revisions
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "[].{Name:name, Active:properties.active, Created:properties.createdTime, Image:properties.template.containers[0].image}" \
  -o table

# Check current image
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.template.containers[0].image" \
  -o tsv
```

### Application Insights

View metrics in Azure Portal:
1. Navigate to Application Insights resource
2. Check performance, failures, dependencies
3. Query logs with Kusto Query Language (KQL)

## Infrastructure Components

### Created Resources (via Terraform)

| Resource | Purpose | Environment |
|----------|---------|-------------|
| **Container Registry** | Stores Docker images | `unitoneagw{env}acr` |
| **Key Vault** | Stores OAuth secrets | `unitone-agw-{env}-kv` |
| **Container App Env** | Hosts container apps | `unitone-agw-{env}-env` |
| **Container App** | Runs AgentGateway | `unitone-agw-{env}-app` |
| **Log Analytics** | Centralized logging | `unitone-agw-{env}-logs` |
| **App Insights** | Application monitoring | `unitone-agw-{env}-insights` |
| **ACR Task** | Automated builds | `agentgateway-build-task` |

### Scaling Configuration

- **Dev**: 1-3 replicas
- **Staging**: 1-5 replicas
- **Prod**: 2-10 replicas

Auto-scaling based on HTTP concurrent requests (100 per replica).

## Troubleshooting

### Build Not Triggering

```bash
# Check ACR Task exists
az acr task show --registry unitoneagwdevacr --name agentgateway-build-task

# If missing, run terraform apply
cd /path/to/terraform/repo/environments/dev/agentgateway
terraform apply
```

### Container App Not Starting

```bash
# Check replica status
az containerapp replica list \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --output table

# View container logs
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --tail 50
```

### OAuth Secrets Not Found

```bash
# Check secrets in Key Vault
az keyvault secret list --vault-name unitone-agw-dev-kv --query "[].name" -o table

# Verify Container App can access Key Vault
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "identity.principalId"

# Check access policy
az keyvault show \
  --name unitone-agw-dev-kv \
  --query "properties.accessPolicies[?objectId=='<PRINCIPAL_ID>']"
```

### OAuth Flow Not Working

1. **Check redirect URIs** match your deployment URL
2. **Verify secrets** are correctly configured in Terraform
3. **Check CORS settings** - configured in Terraform
4. **Test token validation** with your OAuth provider's introspection endpoint
5. **Review logs** for authentication errors

## Security Best Practices

1. **Rotate Secrets Regularly**:
   ```bash
   # Update in Terraform
   cd /path/to/terraform/repo/environments/dev/agentgateway

   # Edit terraform.tfvars with new secrets
   vim terraform.tfvars

   # Apply changes
   terraform apply

   # Container App will automatically restart with new secrets
   ```

2. **Use Managed Identities**: Already configured in Terraform

3. **Enable HTTPS Only**: Configured in ingress settings

4. **Restrict CORS Origins**: Update in Terraform configuration for production

5. **Monitor Access**: Use Application Insights to track OAuth failures

6. **Secure terraform.tfvars**: Never commit to git, use `.gitignore`

## Cost Estimate

| Environment | Monthly Cost |
|-------------|--------------|
| **Dev** | ~$15-30 |
| **Staging** | ~$30-60 |
| **Prod** | ~$100-300 |

Costs include: Container Apps, ACR, Key Vault, Log Analytics, Application Insights.

## Environment Details

### Dev Environment
- **Resource Group**: `mcp-gateway-dev-rg`
- **ACR**: `unitoneagwdevacr`
- **Container App**: `unitone-agw-dev-app`
- **Auto-deploy**: Enabled on push to `main`
- **Scaling**: 1-3 replicas

### Production Environment
- **Resource Group**: `mcp-gateway-prod-rg` (if configured)
- **ACR**: `unitoneagwprodacr`
- **Container App**: `unitone-agw-prod-app`
- **Auto-deploy**: Manual/controlled via Terraform
- **Scaling**: 2-10 replicas

## Development Workflow

1. Make code changes in feature branch
2. Create PR to `main`
3. PR triggers CI tests (GitHub Actions)
4. After approval, merge to `main`
5. ACR Task automatically builds new image
6. Container App auto-deploys (if enabled)
7. Verify deployment

## Reference Documentation

- **Terraform Configuration**: See `terraform` repository
  - Module: `modules/azure/agentgateway/`
  - CI/CD: `modules/azure/agentgateway/ci_cd.tf`
  - Environment: `environments/dev/agentgateway/`
- **Deployment Guide**: See `deploy/DEPLOYMENT.md` in this repository
- **ACR Tasks**: https://docs.microsoft.com/azure/container-registry/container-registry-tasks-overview
- **Container Apps**: https://docs.microsoft.com/azure/container-apps/
- **Terraform Azure Provider**: https://registry.terraform.io/providers/hashicorp/azurerm/latest/docs

## Next Steps

1. **Configure OAuth Providers**: Set up redirect URIs in each provider
2. **Customize Configuration**: Edit `azure-config.yaml` for your MCP servers
3. **Set up Monitoring**: Configure alerts in Application Insights
4. **Add More Environments**: Create staging/prod environments in Terraform
5. **Enable Rate Limiting**: Add rate limit policies in config
6. **Custom Domain**: Configure custom domain in Terraform

## Support

For issues or questions:
- Check [AgentGateway Documentation](https://github.com/agentgateway/agentgateway)
- Review [MCP Authentication Spec](https://spec.modelcontextprotocol.io/specification/2025-11-05/authentication/)
- Check deployment docs in `deploy/DEPLOYMENT.md`
- Open an issue on GitHub

---

**Created by**: UnitOne DevOps Team
**Last Updated**: January 2026
