# UnitOne User Menu Feature - Branch Summary

**Branch:** `unitone/user-menu-feature`
**Base:** `main`
**Commit:** `f2c34a7`
**Date:** December 23, 2025

## Overview

This branch adds an authenticated user menu component with Microsoft and Google OAuth integration using Azure Container Apps Easy Auth. The feature includes complete infrastructure-as-code setup using Azure Bicep.

**⚠️ IMPORTANT:** This is a UnitOne-specific customization and is NOT intended for contribution back to the upstream agentgateway project.

## What Was Changed

### Statistics
- **22 files changed**
- **2,856 insertions**, 4 deletions
- **1 commit**

###Application Code (UI)

#### New Files Added
1. **`ui/src/hooks/use-auth.ts`** (114 lines)
   - Custom React hook to integrate with Easy Auth
   - Fetches user info from `/.auth/me` endpoint
   - Parses OAuth claims (name, email, provider)
   - Provides logout function

2. **`ui/src/components/user-menu.tsx`** (126 lines)
   - User menu component with avatar and dropdown
   - Displays user name, email, and provider
   - Provider-specific SVG icons (Microsoft 4-color, Google colored)
   - Sign out button

3. **`ui/src/components/ui/avatar.tsx`** (47 lines)
   - Radix UI Avatar component
   - Displays user initials in sidebar

4. **`ui/theme.config.ts`** (95 lines)
   - Theme configuration for UnitOne branding
   - Customizable name and tagline

5. **`ui/public/icon.svg`** & **`ui/public/images/unitone-logo.png`**
   - UnitOne branding assets

#### Modified Files
1. **`ui/src/components/app-sidebar.tsx`**
   - Integrated UserMenu component in sidebar footer
   - Updated header to use theme config branding

2. **`ui/package.json`**
   - Added `@radix-ui/react-avatar` dependency

3. **`ui/package-lock.json`**
   - Dependency lock file updated

### Infrastructure Code (Bicep)

#### New Infrastructure Files
1. **`deploy/bicep/main.bicep`** (385 lines)
   - Complete Azure infrastructure definition
   - Resources: ACR, Log Analytics, App Insights, Container App Environment, Container App, Key Vault
   - Easy Auth configuration included

2. **`deploy/bicep/parameters-dev.json`** (77 lines)
   - Development environment parameters
   - Key Vault references for OAuth secrets

3. **`deploy/bicep/parameters-prod.json`** (77 lines)
   - Production environment parameters

4. **`deploy/deploy.sh`** (214 lines)
   - Automated deployment script
   - Supports `--environment`, `--build`, `--tag` flags

5. **`deploy/configs/oauth-config.yaml`** & **`deploy/configs/hybrid-oauth-config.yaml`**
   - AgentGateway OAuth configuration examples
   - MCP server routes with OAuth policies

#### Documentation Files
1. **`deploy/EASY_AUTH_DEPLOYMENT.md`** (111 lines)
   - Comprehensive Easy Auth deployment documentation
   - Azure CLI commands for configuration

2. **`deploy/README.md`** (347 lines)
   - Complete deployment guide
   - Infrastructure setup and troubleshooting

3. **`AZURE_PORTAL_OAUTH_GUIDE.md`** (294 lines)
   - Step-by-step Azure Portal configuration
   - Screenshots and detailed instructions

4. **`QUICK_START.md`** (107 lines)
   - Developer quick start guide

5. **`configure-easy-auth.sh`** (184 lines)
   - Script to configure Easy Auth settings

#### Deployment Files
1. **`Dockerfile.acr`** (108 lines)
   - Azure Container Registry optimized Dockerfile
   - Multi-stage build with UI and Rust backend

2. **`deploy/easy-auth-config.json`** (33 lines)
   - Current Easy Auth configuration snapshot

## Azure Resources

### Created Resources
- **Subscription:** `398d3082-298e-4633-9a4a-816c025965ee`
- **Resource Group:** `mcp-gateway-dev-rg`
- **Location:** `East US 2`
- **Container Registry:** `agwimages`
- **Container App:** `unitone-agentgateway`
- **Managed Environment:** `unitone-agw-env`

### OAuth Providers Configured

#### Microsoft (Azure AD)
- **Client ID:** `4b497d98-cb3a-400e-9374-0e23d57dd480`
- **Tenant ID:** `2cbdd19f-2624-461f-9901-bd63473655a7`
- **App Registration:** UNITONE Gateway
- **Redirect URI:** `https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/.auth/login/aad/callback`

#### Google OAuth 2.0
- **Client ID:** `919355621898-us1vie0rv5mqaff752hhqb9espne87ug.apps.googleusercontent.com`
- **Redirect URI:** `https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/.auth/login/google/callback`

## How to Deploy

### Option 1: Automated Script
```bash
cd /Users/surindersingh/source_code/agentgateway
./deploy/deploy.sh --environment dev --build --tag latest
```

### Option 2: Manual Deployment

**1. Build Docker Image:**
```bash
az acr build \
  --registry agwimages \
  --image unitone-agentgateway:latest \
  --file Dockerfile.acr \
  --platform linux/amd64 .
```

**2. Deploy to Container App:**
```bash
az containerapp update \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --image agwimages.azurecr.io/unitone-agentgateway:latest
```

**3. Configure Easy Auth (if needed):**
```bash
./configure-easy-auth.sh
```

### Option 3: Infrastructure from Scratch
```bash
az deployment group create \
  --resource-group mcp-gateway-dev-rg \
  --template-file deploy/bicep/main.bicep \
  --parameters deploy/bicep/parameters-dev.json
```

## Testing

### 1. Logout (Clear Session)
```
https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/.auth/logout
```

### 2. Access UI (Should Show Login)
```
https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/ui
```

### 3. Check Auth Info
```
https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/.auth/me
```

### 4. Verify User Menu
- After login, check sidebar footer for user avatar
- Click avatar to see dropdown menu with name, email, provider icon, and sign out button

## Architecture

### Authentication Flow
1. User accesses `/ui`
2. **Azure Container Apps Easy Auth** intercepts the request
3. If not authenticated, redirects to Microsoft/Google login
4. After successful OAuth, Easy Auth:
   - Creates session cookie
   - Makes user claims available at `/.auth/me`
5. UI application:
   - Calls `/.auth/me` to fetch user info
   - Displays user menu with avatar and details
6. Sign out redirects to `/.auth/logout`

### Key Design Decisions
- **Platform-Level Auth:** No application OAuth code - handled entirely by Azure Container Apps
- **Stateless UI:** UI doesn't manage tokens or sessions
- **Radix UI Components:** Accessible, customizable components
- **Infrastructure as Code:** Complete Bicep setup for reproducibility
- **Isolated Branch:** Kept separate to avoid upstream conflicts

## Files Not Included
These branding-related files are modified but NOT included in the commit to keep it focused:
- `Dockerfile` (main file, has other changes)
- `ui/next.config.ts`
- `ui/src/app/globals.css`
- `ui/src/app/layout.tsx`
- `ui/src/components/agentgateway-logo.tsx`
- `ui/src/hooks/use-xds-mode.ts`
- `ui/src/lib/api.ts`

## Comparison with Main

To view the full diff:
```bash
git diff main..unitone/user-menu-feature
```

To view specific file changes:
```bash
git diff main..unitone/user-menu-feature -- ui/src/hooks/use-auth.ts
git diff main..unitone/user-menu-feature -- ui/src/components/user-menu.tsx
git diff main..unitone/user-menu-feature -- deploy/bicep/main.bicep
```

## Merging This Branch

**⚠️ DO NOT MERGE TO MAIN**

This branch should be kept separate as it contains Unit One-specific customizations. To maintain this feature:

### Option 1: Keep as Permanent Branch
```bash
# Always deploy from this branch
git checkout unitone/user-menu-feature
./deploy/deploy.sh --environment prod
```

### Option 2: Rebase on Main
```bash
# When upstream changes, rebase this branch
git checkout unitone/user-menu-feature
git fetch origin
git rebase origin/main
```

### Option 3: Fork Repository
Consider maintaining a UnitOne fork of agentgateway with these customizations.

## Troubleshooting

### Issue: Login screen not appearing
**Solution:** Clear browser cache and cookies, or use incognito mode

### Issue: User menu not visible after login
**Solution:**
1. Check browser console for errors
2. Verify `/.auth/me` returns user data
3. Hard refresh (Ctrl+Shift+R / Cmd+Shift+R)

### Issue: OAuth redirect errors
**Solution:**
1. Verify redirect URIs in Azure Portal
2. Check Easy Auth configuration: `az containerapp auth show`
3. Confirm client secrets are set correctly

## Contact & Support

For issues specific to this feature:
- Check `deploy/README.md` for troubleshooting
- Review Azure Container App logs: `az containerapp logs show`
- Verify Easy Auth config: See `deploy/EASY_AUTH_DEPLOYMENT.md`

---

**Generated:** December 23, 2025
**Branch:** unitone/user-menu-feature
**Commit:** f2c34a7
