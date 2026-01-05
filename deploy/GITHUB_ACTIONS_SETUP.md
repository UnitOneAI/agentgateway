# GitHub Actions Setup Guide

This guide will help you set up automated Azure deployments using GitHub Actions.

## Overview

Once configured, GitHub Actions will:
- **Auto-deploy to dev** on every push to `main`
- **Auto-deploy to prod** when you publish a release
- Build Docker images with automatic tagging (commit SHA for dev, version for prod)
- Run health checks after deployment
- Output deployment URLs

---

## Step 1: Create Azure Service Principal

Create an Azure Service Principal that GitHub Actions will use to deploy to Azure:

```bash
# Replace with your subscription ID
SUBSCRIPTION_ID="<your-subscription-id>"

# Create service principal with contributor access to dev resource group
az ad sp create-for-rbac \
  --name "github-actions-agentgateway" \
  --role contributor \
  --scopes /subscriptions/$SUBSCRIPTION_ID/resourceGroups/unitone-agw-dev-rg \
  --sdk-auth
```

**Copy the entire JSON output** - you'll need it in the next step.

The output looks like this:
```json
{
  "clientId": "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX",
  "clientSecret": "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "subscriptionId": "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX",
  "tenantId": "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX",
  "activeDirectoryEndpointUrl": "https://login.microsoftonline.com",
  "resourceManagerEndpointUrl": "https://management.azure.com/",
  "activeDirectoryGraphResourceId": "https://graph.windows.net/",
  "sqlManagementEndpointUrl": "https://management.core.windows.net:8443/",
  "galleryEndpointUrl": "https://gallery.azure.com/",
  "managementEndpointUrl": "https://management.core.windows.net/"
}
```

**Security Note**: Keep this JSON secure! It provides full access to your Azure resource group.

---

## Step 2: Add GitHub Secret

1. **Go to your GitHub repository**
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `AZURE_CREDENTIALS`
5. Value: **Paste the entire JSON output from Step 1**
6. Click **Add secret**

---

## Step 3: Verify Resource Groups Exist

Make sure your Azure resource groups exist for the environments you want to deploy to:

### Dev Environment (Required for auto-deploy)
```bash
az group show --name unitone-agw-dev-rg
```

If it doesn't exist, create it:
```bash
az group create --name unitone-agw-dev-rg --location eastus2
```

### Prod Environment (Optional - for release deployments)
```bash
az group show --name unitone-agw-prod-rg
```

If you want prod auto-deployment, create it:
```bash
az group create --name unitone-agw-prod-rg --location eastus2
```

---

## Step 4: Grant Service Principal Access to Prod (Optional)

If you created a prod environment, grant the service principal access:

```bash
# Get the service principal's client ID from the JSON in Step 1
CLIENT_ID="<client-id-from-step-1>"
SUBSCRIPTION_ID="<your-subscription-id>"

az role assignment create \
  --assignee $CLIENT_ID \
  --role contributor \
  --scope /subscriptions/$SUBSCRIPTION_ID/resourceGroups/unitone-agw-prod-rg
```

---

## Step 5: Test the Workflow

### Option A: Test with a small change (recommended)

1. Make a small change to README.md or add a comment
2. Commit and push to `main`:
   ```bash
   git add .
   git commit -m "test: trigger GitHub Actions deployment"
   git push origin main
   ```

3. **Watch the deployment**:
   - Go to your repository on GitHub
   - Click **Actions** tab
   - You should see "Azure Deployment" workflow running
   - Click on the running workflow to see detailed logs

4. **Check the output**:
   - At the end of the workflow, you'll see deployment URLs
   - UI URL: `https://<app-name>.azurecontainerapps.io/ui`
   - MCP Endpoint: `https://<app-name>.azurecontainerapps.io/mcp`

### Option B: Manual workflow dispatch

1. Go to **Actions** tab on GitHub
2. Click **Azure Deployment** workflow
3. Click **Run workflow**
4. Select environment: `dev`
5. (Optional) Enter custom tag
6. Click **Run workflow**

---

## Step 6: Create a Release (Prod Deployment)

When you're ready to deploy to production:

1. **Create a git tag**:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. **Create a GitHub release**:
   - Go to your repository → **Releases** → **Create a new release**
   - Choose the tag: `v1.0.0`
   - Add release title: "v1.0.0"
   - Add description of changes
   - Click **Publish release**

3. **Watch the production deployment**:
   - Go to **Actions** tab
   - You'll see "Azure Deployment" workflow deploying to prod
   - The Docker image will be tagged with `1.0.0` (without the 'v')

---

## Workflow Behavior

### Dev Deployment (Push to Main)

**Trigger**: Every push to `main` branch

**Image Tags Created**:
- `<short-commit-sha>` (e.g., `a1b2c3d`)
- `<timestamp>` (e.g., `20250131-120000`)
- `latest`

**Example**:
```
Commit: a1b2c3d
Tags: a1b2c3d, 20250131-120000, latest
```

### Prod Deployment (Release Published)

**Trigger**: Publishing a GitHub release

**Image Tags Created**:
- `<semantic-version>` (e.g., `1.0.0` from tag `v1.0.0`)
- `<timestamp>` (e.g., `20250131-120000`)
- `latest`

**Example**:
```
Release: v1.0.0
Tags: 1.0.0, 20250131-120000, latest
```

### Manual Deployment (Workflow Dispatch)

**Trigger**: Manual run from GitHub Actions UI

**Image Tags Created**:
- `<custom-tag>` or `<commit-sha>` (depending on what you specify)
- `<timestamp>`
- `latest`

---

## Monitoring Deployments

### GitHub Actions UI

1. Go to **Actions** tab
2. Click on the running/completed workflow
3. View logs for each step:
   - Build and push to ACR
   - Deploy to Azure Container Apps
   - Verify deployment health
   - Get deployment outputs

### Azure Portal

1. **Container App**:
   - Go to: Resource Groups → `unitone-agw-dev-rg` → `unitone-agw-dev-app`
   - View: Revisions, Logs, Metrics

2. **Application Insights**:
   - View telemetry, errors, performance

3. **Log Analytics**:
   - Run KQL queries across all components

### Azure CLI

```bash
# Follow logs
az containerapp logs show \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --follow

# Check health
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --query "{ProvisioningState:properties.provisioningState, RunningState:properties.runningStatus}"

# List revisions
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group unitone-agw-dev-rg \
  --query "[].{Name:name, Active:properties.active, Image:properties.template.containers[0].image, Traffic:properties.trafficWeight}" \
  -o table
```

---

## Troubleshooting

### Workflow Fails: "Not Authorized"

**Problem**: Service principal doesn't have access

**Solution**:
1. Verify `AZURE_CREDENTIALS` secret is set correctly
2. Check service principal has Contributor role:
   ```bash
   az role assignment list \
     --assignee <client-id> \
     --scope /subscriptions/<subscription-id>/resourceGroups/unitone-agw-dev-rg
   ```
3. If missing, re-run Step 1 or grant access manually:
   ```bash
   az role assignment create \
     --assignee <client-id> \
     --role contributor \
     --scope /subscriptions/<subscription-id>/resourceGroups/unitone-agw-dev-rg
   ```

### Image Build Fails

**Problem**: `az acr build` fails

**Solution**:
1. Check ACR exists:
   ```bash
   az acr show --name unitoneagwdevacr
   ```
2. Check Dockerfile.acr is valid
3. Check build logs in GitHub Actions for specific error

### Container App Not Starting

**Problem**: Deployment succeeds but app doesn't start

**Solution**:
1. Check container logs:
   ```bash
   az containerapp logs show \
     --name unitone-agw-dev-app \
     --resource-group unitone-agw-dev-rg \
     --tail 100
   ```
2. Check environment variables are set correctly
3. Check image was pushed to ACR:
   ```bash
   az acr repository show-tags \
     --name unitoneagwdevacr \
     --repository unitone-agentgateway
   ```

### Deployment Completes but UI Returns 404

**Problem**: UI endpoint returns 404 or 406

**Solution**:
1. Verify `/ui` route is configured in `azure-config.yaml`
2. Check recent deployment included the updated config
3. Restart the container app:
   ```bash
   az containerapp revision restart \
     --name unitone-agw-dev-app \
     --resource-group unitone-agw-dev-rg \
     --revision <latest-revision-name>
   ```

---

## Next Steps

Once GitHub Actions is set up:

1. **Commit your changes** with the UI fix and automation:
   ```bash
   git add .
   git commit -m "feat: add UI routing and GitHub Actions automation"
   git push origin main
   ```

2. **Watch the auto-deployment** in the Actions tab

3. **Access your UI** at the URL shown in the workflow output

4. **Review `deploy/DEPLOYMENT.md`** for comprehensive deployment documentation

5. **Create your first release** when ready to deploy to production

---

## Summary

You've now set up:
- ✅ Automated dev deployments on push to `main`
- ✅ Automated prod deployments on release
- ✅ Manual deployment option via workflow dispatch
- ✅ Automatic image tagging and versioning
- ✅ Health checks after deployment
- ✅ Deployment URL outputs

No more manual `az acr build` or `az containerapp update` commands needed!
