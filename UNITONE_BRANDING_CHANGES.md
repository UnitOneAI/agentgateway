# UnitOne Branding Changes

This document details all the changes made to apply UnitOne branding to agentgateway.

## Summary

The agentgateway UI has been rebranded to use the UnitOne theme with:
- **Blue color scheme** (#3b82f6 instead of purple #7734be)
- **Dark sidebar** (#0f172a) and dark header (#020618)
- **UnitOne logo** instead of the agentgateway SVG logo
- **Inter font** instead of Geist
- **Centralized theme configuration** for easy future rebranding

## Files Changed

### 1. **New File: `ui/theme.config.ts`**
**Purpose:** Central configuration file for all branding settings

**What it contains:**
- Brand name: "UnitOne"
- Tagline: "Agent Security"
- Logo configuration (path, dimensions)
- Complete color palette
- Font settings

**Why it matters:** Change branding in ONE file instead of hunting through multiple components.

---

### 2. **Modified: `ui/src/app/globals.css`**
**Changes:**
- Primary color: `#7734be` → `#3b82f6` (blue)
- Sidebar colors: Light background → Dark background (#0f172a)
- Border colors: Light → Dark (#1e293b)
- Chart colors: Updated to match UnitOne palette
- Added comments referencing theme.config.ts

**Lines changed:**
- Line 10: Added comment about UnitOne theme
- Lines 19, 25, 31: Changed primary/accent/ring colors to #3b82f6
- Lines 34-42: Updated sidebar colors to dark theme
- Lines 85-122: Updated oklch color values for light mode
- Lines 124-159: Updated oklch color values for dark mode

---

### 3. **Modified: `ui/src/app/layout.tsx`**
**Changes:**
- Font: `Geist` → `Inter` (with weights 400, 500, 600, 700)
- Title: "Agentgateway Dashboard" → "UnitOne - Agent Security Platform"
- Description: Updated to "Enterprise-grade agentic AI security platform"
- Storage key: "agentgateway-theme" → "unitone-theme"
- Font variable: `geistSans` → `inter`

**Lines changed:**
- Line 2: Import changed to Inter
- Lines 15-19: Font configuration
- Lines 22-26: Metadata updated
- Line 37: Body className updated
- Line 45: Theme storage key updated

---

### 4. **Modified: `ui/src/components/agentgateway-logo.tsx`**
**Changes:**
- **Complete rewrite:** SVG → Image component
- Now uses `theme.config.ts` for logo settings
- Supports custom width/height props
- Uses Next.js Image optimization
- Logo path: `/images/unitone-logo.png`

**Result:** Logo can be changed by updating `theme.config.ts` only

---

### 5. **Modified: `ui/src/components/app-sidebar.tsx`**
**Changes:**
- Import: Added `themeConfig` from theme.config.ts
- Header layout: Updated to match UnitOne design
- Logo: Changed from `h-10` to `h-9 w-9` with explicit dimensions
- Text: "agentgateway" → Dynamic from `themeConfig.branding.name`
- Added tagline: `themeConfig.branding.tagline` ("Agent Security")
- Border styling: Added `border-sidebar-border` class

**Lines changed:**
- Line 34: Added themeConfig import
- Lines 90-98: Sidebar header restructured

---

### 6. **New File: `ui/public/images/unitone-logo.png`**
**What:** 731KB PNG logo copied from agent-security-control-plane
**Path:** `/ui/public/images/unitone-logo.png`
**Used by:** Logo component via theme.config.ts

---

## Color Changes at a Glance

| Element | Before | After |
|---------|--------|-------|
| Primary | Purple #7734be | Blue #3b82f6 |
| Sidebar Background | Light #f8fafc | Dark #0f172a |
| Sidebar Border | Light #e5e7eb | Dark #1e293b |
| Header Background | (Not defined) | Dark Navy #020618 |
| Chart Colors | Mixed | Blue, Green, Yellow, Red, Purple |

---

## How to Change Branding in the Future

### Option 1: Edit `theme.config.ts` (Recommended)
```typescript
// Change everything in one place:
export const themeConfig = {
  branding: {
    name: "YourCompany",           // ← Company name
    tagline: "Your Tagline",       // ← Tagline
    logo: {
      image: "/images/your-logo.png",  // ← Logo path
      width: 36,
      height: 36,
    },
  },
  colors: {
    primary: "#your-color",        // ← Primary brand color
    sidebar: "#your-sidebar-color",
    // ... etc
  },
}
```

### Option 2: Replace Logo Image
```bash
# Just replace the file:
cp your-logo.png ui/public/images/unitone-logo.png

# Or change the path in theme.config.ts
```

### Option 3: Change Colors
Edit `ui/src/app/globals.css`:
- Find `#3b82f6` and replace with your color
- Update sidebar/nav colors if needed

---

## Build Instructions

After making branding changes:

```bash
# 1. Build the UI
cd ui
npm install
npm run build

# 2. Build the Rust binary with embedded UI
cd ..
cargo build --release --features ui

# 3. Or use Docker
docker build -t yourcompany/agentgateway:v1.0.0 .
```

---

## Testing the Changes

```bash
# Run in development mode (with hot reload):
cd ui
npm run dev
# Visit http://localhost:3000/ui

# Or run the full agentgateway binary:
cargo run --features ui -- --config examples/config.yaml
# Visit http://localhost:19000/ui
```

---

## Reverting to Original Branding

If you want to revert to the original agentgateway branding:

1. **Restore `theme.config.ts`:**
   ```typescript
   branding: {
     name: "Agentgateway",
     tagline: "Gateway",
     // ... etc
   }
   colors: {
     primary: "#7734be",  // Purple
     // ... etc
   }
   ```

2. **Restore the original logo:**
   - Delete `/ui/public/images/unitone-logo.png`
   - Or change `agentgateway-logo.tsx` back to SVG

3. **Update `globals.css`:**
   - Replace all `#3b82f6` with `#7734be`
   - Revert sidebar colors to light theme

---

## Architecture Notes

### Why This Approach?

**Centralized Configuration:**
- All branding in one file (`theme.config.ts`)
- Easy to maintain and update
- Can be version controlled separately

**Type Safety:**
- TypeScript ensures branding values are consistent
- Compile-time checks prevent typos

**Component Reusability:**
- Components reference `themeConfig` instead of hardcoded values
- Easier to create multiple brand variants

**Build-Time Embedding:**
- UI is embedded in the Rust binary at compile time
- No runtime dependencies on external assets
- Single binary deployment

### Limitations

**Cannot change UI at runtime:**
- Must rebuild the binary to change branding
- UI is embedded at compile time via `include_dir!` macro

**To enable runtime UI changes:**
- Modify `crates/agentgateway/src/ui.rs` to load from filesystem
- Add environment variable: `EXTERNAL_UI_PATH`
- Trade-off: More complex deployment

---

## Git Diff Summary

```diff
 ui/theme.config.ts                          | 100 +++ (NEW FILE)
 ui/src/app/globals.css                      |  68 +++---
 ui/src/app/layout.tsx                       |  24 +++---
 ui/src/components/agentgateway-logo.tsx     |  50 ++---
 ui/src/components/app-sidebar.tsx           |  10 +++---
 ui/public/images/unitone-logo.png           | BIN (NEW FILE)
 UNITONE_BRANDING_CHANGES.md                 | 282 +++ (THIS FILE)
```

---

## Next Steps

1. **Test the build:**
   ```bash
   cd ui && npm run build && cd .. && cargo build --features ui
   ```

2. **Verify branding:**
   - Logo appears correctly
   - Colors match UnitOne theme
   - Dark sidebar renders properly
   - Title and metadata updated

3. **Create Docker image:**
   ```bash
   docker build -t yourcompany/agentgateway:unitone-v1.0.0 .
   ```

4. **Deploy to your environment**

---

## Contact

For questions about these branding changes, refer to:
- `ui/theme.config.ts` - Central configuration
- This document - Change history
- Original agentgateway docs: https://agentgateway.dev/docs

---

**Generated:** 2025-12-11
**Agentgateway Version:** 0.7.0
**Theme:** UnitOne Agent Security Platform
