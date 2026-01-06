# Build and Deploy Dependency Management

## Current State: Fully Automated via Terraform

**Deployment is now fully automated via Terraform!** The manual workflow described below is for special cases only (debugging, testing specific tags, etc.).

### How It Works Now

1. **Push to `main`** (or merge PR)
2. **ACR Task auto-triggers** (configured via Terraform in separate `terraform` repository)
3. **Build completes** and image is pushed to ACR
4. **Container App auto-deploys** (if `enable_auto_deployment = true` in Terraform config)
5. **Done!** No manual intervention needed.

**Terraform manages:**
- ACR Task creation and configuration
- GitHub webhook setup
- Automatic builds on git push
- Automatic deployments on successful build

**See**: `terraform/modules/azure/agentgateway/ci_cd.tf` in the Terraform repository

---

## Manual Workflow (For Special Cases)

The script below is for situations where you need manual control over the build/deploy process:
- Testing specific feature branches
- Debugging deployment issues
- Deploying with custom tags
- Bypassing automated pipeline

### Problem (That Was Solved)

Previously, deployments would run before ACR builds completed, resulting in:
- Deployments using old/wrong images
- E2E tests running against outdated code
- Wasted time debugging "fixes" that weren't actually deployed

### Solution

The `wait_for_build_and_deploy.sh` script ensures proper dependency management by:
1. Polling ACR task logs until build completes (success or failure)
2. Only deploying after build succeeds
3. Waiting for deployment stabilization
4. Showing active revision confirmation

## Manual Usage

### Basic Usage
```bash
cd /Users/surindersingh/source_code/agentgateway/tests
./wait_for_build_and_deploy.sh <run-id> <image-tag>
```

### With Custom Resource Group and App Name
```bash
./wait_for_build_and_deploy.sh <run-id> <image-tag> <resource-group> <app-name>
```

### Example Manual Workflow

1. **Start ACR build manually:**
```bash
cd /Users/surindersingh/source_code/agentgateway
az acr build --registry unitoneagwdevacr \
  --image unitone-agentgateway:latest \
  --image unitone-agentgateway:my-feature \
  --file Dockerfile.acr \
  --platform linux/amd64 \
  .
```

2. **Note the run ID from output** (e.g., `ch1c`)

3. **Use wait script to deploy:**
```bash
cd tests
./wait_for_build_and_deploy.sh ch1c my-feature
```

4. **Run E2E tests:**
```bash
./test_venv/bin/python3 e2e_mcp_sse_test.py
```

## Parameters

- **run-id** (required): ACR build run ID (e.g., ch1c)
- **image-tag** (required): Image tag to deploy (e.g., my-feature)
- **resource-group** (optional): Azure resource group (default: mcp-gateway-dev-rg)
- **app-name** (optional): Container app name (default: unitone-agw-dev-app)

## Script Features

- ✅ Polls build status every 15 seconds
- ✅ Exits immediately on build failure with error logs
- ✅ Waits for "Succeeded" status before deploying
- ✅ Automatic 30-second stabilization wait after deployment
- ✅ Shows active revision confirmation
- ✅ Configurable via parameters
- ✅ Clear status messages throughout process

## Monitoring Automated Builds

Since builds are now automated, here's how to monitor them:

### Check Recent Builds
```bash
az acr task list-runs --registry unitoneagwdevacr -o table
```

### Follow Live Build Logs
```bash
az acr task logs --registry unitoneagwdevacr --name agentgateway-build-task --follow
```

### Check Latest Run
```bash
az acr task list-runs --registry unitoneagwdevacr --name agentgateway-build-task --top 1 -o table
```

### View Specific Build
```bash
az acr task logs --registry unitoneagwdevacr --run-id <run-id>
```

## Deployment Status

### Check Container App Revisions
```bash
az containerapp revision list \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "[].{Name:name, Active:properties.active, Created:properties.createdTime, TrafficWeight:properties.trafficWeight, Image:properties.template.containers[0].image}" \
  -o table
```

### Check Current Image
```bash
az containerapp show \
  --name unitone-agw-dev-app \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.template.containers[0].image" \
  -o tsv
```

## Troubleshooting Automated Deployments

### Build Triggered But Not Deploying

Check if auto-deployment is enabled in Terraform:
```bash
cd /path/to/terraform/repo
cd environments/dev/agentgateway
grep enable_auto_deployment terraform.tfvars
```

### Build Failed

View build logs:
```bash
az acr task logs --registry unitoneagwdevacr --name agentgateway-build-task
```

Common issues:
- GitHub PAT expired (set in Terraform)
- Dockerfile.acr syntax error
- Build dependencies missing

### ACR Task Not Triggering

Verify ACR Task exists and is configured:
```bash
az acr task show --registry unitoneagwdevacr --name agentgateway-build-task
```

If missing, run `terraform apply` in the Terraform repository.

## When to Use Manual Workflow

Use the manual script (`wait_for_build_and_deploy.sh`) when:

1. **Testing feature branches**: Build and deploy specific tags without merging to `main`
2. **Debugging**: Isolate build vs deploy issues
3. **Emergency rollback**: Deploy specific previous image tag
4. **Development iteration**: Rapid testing without git commits

For normal development, **let the automated pipeline handle it!**

## Reference

- **Automated CI/CD Configuration**: `terraform/modules/azure/agentgateway/ci_cd.tf`
- **Deployment Documentation**: See `deploy/DEPLOYMENT.md`
- **ACR Tasks**: https://docs.microsoft.com/azure/container-registry/container-registry-tasks-overview
