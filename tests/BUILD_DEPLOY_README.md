# Build and Deploy Dependency Management

## Problem

Previously, deployments would often run before ACR builds completed, resulting in:
- Deployments using old/wrong images
- E2E tests running against outdated code
- Wasted time debugging "fixes" that weren't actually deployed

## Solution

The `wait_for_build_and_deploy.sh` script ensures proper dependency management by:
1. Polling ACR task logs until build completes (success or failure)
2. Only deploying after build succeeds
3. Waiting for deployment stabilization
4. Showing active revision confirmation

## Usage

### Basic Usage
```bash
cd /Users/surindersingh/source_code/agentgateway/tests
./wait_for_build_and_deploy.sh <run-id> <image-tag>
```

### With Custom Resource Group and App Name
```bash
./wait_for_build_and_deploy.sh <run-id> <image-tag> <resource-group> <app-name>
```

### Example Workflow

1. **Start ACR build:**
```bash
cd /Users/surindersingh/source_code/agentgateway
az acr build --registry agwimages \
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
- **image-tag** (required): Image tag to deploy (e.g., sse-headers-fixed)
- **resource-group** (optional): Azure resource group (default: mcp-gateway-dev-rg)
- **app-name** (optional): Container app name (default: unitone-agentgateway)

## Features

- ✅ Polls build status every 15 seconds
- ✅ Exits immediately on build failure with error logs
- ✅ Waits for "Succeeded" status before deploying
- ✅ Automatic 30-second stabilization wait after deployment
- ✅ Shows active revision confirmation
- ✅ Configurable via parameters
- ✅ Clear status messages throughout process

## Future: Terraform Integration

Once verified working, this logic should be codified in Terraform for automated CI/CD pipelines.

### Recommended Terraform Approach

```hcl
# Build image
resource "null_resource" "acr_build" {
  provisioner "local-exec" {
    command = "az acr build ..."
  }
}

# Wait for build
resource "null_resource" "wait_for_build" {
  depends_on = [null_resource.acr_build]

  provisioner "local-exec" {
    command = "./wait_for_build_and_deploy.sh ${var.run_id} ${var.image_tag}"
  }
}
```

## Legacy Scripts

- `wait_and_deploy.sh` - Original script hardcoded for build ch13
- Should be replaced with the new generic script
