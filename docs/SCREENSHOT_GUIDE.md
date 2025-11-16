# Screenshot Guide for Claude Code Mux

This guide will help you capture high-quality screenshots for the README and documentation.

## Prerequisites

1. Server must be running: `ccm start`
2. Browser at `http://127.0.0.1:13456`
3. Configuration with real providers and models (not empty)
4. Recommended resolution: 1920x1080 or higher
5. Browser zoom: 100% (default)

## Screenshot Specifications

- **Format**: PNG
- **Location**: `docs/images/`
- **Naming**: Lowercase with hyphens (e.g., `dashboard.png`)
- **Recommended tool**: macOS Screenshot (Cmd+Shift+4), Windows Snipping Tool, or Linux Screenshot

## Screenshots to Capture

### 1. Dashboard Overview (`dashboard.png`)

**URL**: `http://127.0.0.1:13456/?tab=overview`

**What to capture**:
- Full browser window showing Overview tab
- Router Configuration summary card
- Providers list (showing 5-6 providers)
- Models list (showing 3-4 models)
- Top navigation bar with all tabs visible
- "💾 Save to Server" and "🔄 Save & Restart" buttons

**Tips**:
- Make sure data is populated (not "No providers" or "No models")
- Ensure router configuration shows all 4 routing options (Default, Think, Background, WebSearch)

---

### 2. Provider Management (`providers.png`)

**URL**: `http://127.0.0.1:13456/?tab=providers`

**What to capture**:
- Full providers list view
- At least 5-6 provider cards showing:
  - Provider name (e.g., "zai coding plan", "openrouter", "kimi-for-coding", "zenmux", "openai")
  - Provider type
  - Enabled status
  - Edit/Delete buttons
- "Add Provider" button prominently visible

**Tips**:
- Show a good mix of provider types (Anthropic-compatible, OpenAI-compatible)
- Make sure providers are enabled (green checkmark or indicator)

---

### 3. Provider Add Form (`provider-add.png`)

**URL**: `http://127.0.0.1:13456/?tab=providers&view=add`

**What to capture**:
- Full add provider form
- Provider type selection cards showing:
  - Anthropic
  - z.ai
  - **ZenMux** (highlight this as new!)
  - Minimax
  - OpenAI
  - OpenRouter
  - Groq
  - Together AI
  - Fireworks AI
  - Deepinfra
  - Cerebras
  - Nebius
  - NovitaAI
  - Baseten
- Form fields below (Provider Name, API Key, Base URL)
- "Cancel" and "Add Provider" buttons

**Tips**:
- Show the card selection grid prominently
- Don't fill out the form fields (show placeholder text)
- Highlight the modern card-based UI design

---

### 4. Model Mappings (`models.png`)

**URL**: `http://127.0.0.1:13456/?tab=models`

**What to capture**:
- Full models list view
- At least 3-4 model cards showing:
  - Model name (e.g., "glm-4.6", "gpt-5.1", "kimi-for-coding")
  - Provider mappings with priority badges:
    - "Priority 1" badge (blue/primary)
    - "Priority 2" badge (gray/secondary) - for fallback
  - Provider → Actual model mapping
- "Add Model" button

**Example models to show**:
```
glm-4.6
  - Priority 1: zai → glm-4.6
  - Priority 2: openrouter → z-ai/glm-4.6

gpt-5.1
  - Priority 1: zenmux → openai/gpt-5.1

kimi-for-coding
  - Priority 1: kimi-for-coding → claude-sonnet-4-5-20250929
```

**Tips**:
- Show at least one model with fallback (Priority 2)
- Make sure priority badges are visible and colored correctly

---

### 5. Model Add Form (`model-add.png`)

**URL**: `http://127.0.0.1:13456/?tab=models&view=add`

**What to capture**:
- Full add model form
- Model Name input field (placeholder showing)
- Provider Mappings section with:
  - "Priority 1" mapping card (blue/highlighted)
  - Provider dropdown
  - Actual Model input field
  - Mapping controls (up/down/remove arrows)
- "+ Fallback Provider Add" button
- "Cancel" and "Add Model" buttons at bottom

**Tips**:
- Show at least 1-2 mapping cards
- Don't fill out form fields (show placeholders)
- Highlight the drag-to-reorder or priority controls

---

### 6. Router Configuration (`routing.png`)

**URL**: `http://127.0.0.1:13456/?tab=router`

**What to capture**:
- Full router configuration form showing:
  - **Default Model** dropdown (populated)
  - **Think Model** dropdown (populated)
  - **Background Model** dropdown (populated)
  - **WebSearch Model** dropdown (populated)
  - **Auto-map Regex Pattern** input field (NEW FEATURE!)
    - Example value: `^claude-`
    - Helper text explaining the feature
- NO submit button (auto-save feature)
- Optional: Show "✓ Auto-saved" indicator if visible

**Tips**:
- Fill out all dropdown fields with real model names
- Show the Auto-map Regex Pattern field with example value `^claude-`
- Highlight that there's no "Save" button (auto-save feature)
- If possible, trigger auto-save and capture the "✓ Auto-saved" notification

---

### 7. Live Testing Interface (`testing.png`)

**URL**: `http://127.0.0.1:13456/?tab=test`

**What to capture**:
- Full test interface showing:
  - Model selection dropdown
  - Message input textarea with example text
  - "Send Message" and "Clear" buttons
  - Response container showing:
    - Response text
    - Response time
    - Model used
    - Token usage (input/output)

**Tips**:
- Send a test message first, then capture the response
- Example message: "Hello, how are you?"
- Show full response with all metadata
- Make sure response is from a working provider

---

### 8. Auto-save Indicator (Optional) (`auto-save.png`)

**URL**: `http://127.0.0.1:13456/?tab=router`

**What to capture**:
- Router tab with "✓ Auto-saved" notification visible in top-right corner
- Green background notification with checkmark

**How to trigger**:
1. Go to Router tab
2. Change any dropdown value
3. Wait 500ms
4. Capture the green "✓ Auto-saved" notification

**Tips**:
- This is an optional but nice-to-have screenshot
- Shows the real-time auto-save feature in action
- Capture within 2 seconds before notification fades

---

## Image Optimization

After capturing screenshots:

```bash
# Create images directory
mkdir -p docs/images

# Move screenshots
mv ~/Desktop/dashboard.png docs/images/
mv ~/Desktop/providers.png docs/images/
mv ~/Desktop/provider-add.png docs/images/
mv ~/Desktop/models.png docs/images/
mv ~/Desktop/model-add.png docs/images/
mv ~/Desktop/routing.png docs/images/
mv ~/Desktop/testing.png docs/images/

# Optional: Optimize images (requires imageoptim or similar)
# This reduces file size without quality loss
```

## Checklist

Before publishing, ensure:

- [ ] All 7 required screenshots are captured
- [ ] Images are in PNG format
- [ ] Images are in `docs/images/` directory
- [ ] Images show real data (not empty states)
- [ ] Browser UI is clean (no dev tools, bookmarks bar minimal)
- [ ] Text is readable (not too small)
- [ ] No sensitive API keys visible
- [ ] Auto-save feature is highlighted in Router screenshot
- [ ] ZenMux provider is visible in provider selection
- [ ] Fallback priority badges are clearly visible in Models

## Screenshot Summary

| File | Purpose | Key Elements |
|------|---------|--------------|
| `dashboard.png` | Main overview | All 4 routing configs, provider list, model list |
| `providers.png` | Provider management | 5-6 providers with Edit/Delete buttons |
| `provider-add.png` | Add provider UI | Card selection, form fields, ZenMux highlighted |
| `models.png` | Model mappings | 3-4 models with Priority 1/2 badges |
| `model-add.png` | Add model UI | Mapping configuration, fallback button |
| `routing.png` | Router config | All 4 dropdowns + Auto-map Regex field |
| `testing.png` | Live testing | Request/response with metadata |
| `auto-save.png` (optional) | Auto-save feature | Green notification indicator |

---

**Ready to capture? Start your server and follow this guide!**

```bash
ccm start
# Open http://127.0.0.1:13456 and start capturing!
```
