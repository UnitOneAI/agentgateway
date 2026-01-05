#!/bin/bash
# Wait for build ch13 to complete, then deploy and test

cd /Users/surindersingh/source_code/agentgateway/tests

echo "Waiting for build ch13 to complete..."

# Poll until build is complete
while true; do
  STATUS=$(az acr task logs --registry agwimages --run-id ch13 2>&1 | grep "Run Status" | tail -1)
  echo "Current status: $STATUS"

  if echo "$STATUS" | grep -q "Succeeded"; then
    echo "✓ Build ch13 completed successfully!"
    break
  elif echo "$STATUS" | grep -q "Failed"; then
    echo "✗ Build ch13 failed!"
    exit 1
  fi

  sleep 15
done

echo "Deploying fresh image..."
az containerapp update \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --image agwimages.azurecr.io/unitone-agentgateway:latest

echo "✓ Deployment complete!"

echo "Waiting 30 seconds for deployment to stabilize..."
sleep 30

echo "Running E2E tests..."
./test_venv/bin/python3 e2e_mcp_sse_test.py

echo "✓ All tasks completed!"
