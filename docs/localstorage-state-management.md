# LocalStorage-based State Management

The admin UI uses localStorage to manage client-side state and syncs with the server only on explicit save.

## Concept

LocalStorage-based state management uses browser localStorage as a client-side cache to minimize unnecessary communication with the server.

### Why this approach?

The server reads TOML config files and **does not reload them until restart**. Therefore:

1. ❌ **Wrong approach**: Re-fetch from server after each operation
   - Save config to server → Fetch again from server
   - Problem: Server returns **stale data** until restart

2. ✅ **Correct approach**: Use localStorage as client-side cache
   - Fetch from server once on page load → Save to localStorage
   - All operations (add/delete/edit) update localStorage only
   - Sync to server only when user clicks "Save" button

## Architecture

```
Page Load
    ↓
Fetch config from server (/api/config/json)
    ↓
Save to localStorage (cache)
    ↓
Render UI
    ↓
User actions (Add/delete Provider, Add/delete Model)
    ↓
Update localStorage only (NOT server)
    ↓
Update UI immediately
    ↓
User clicks "Save" or "Save & Restart" button
    ↓
Sync localStorage → server (/api/config/json POST)
    ↓
(Optional) Restart server (/api/restart POST)
```

## Implementation

### Global State

```javascript
const appState = {
    config: null,
    loaded: false
};
```

### Core Functions

#### `saveToLocalStorage(config)`
Saves config to localStorage.

```javascript
function saveToLocalStorage(config) {
    try {
        localStorage.setItem('ccm_config', JSON.stringify(config));
        return true;
    } catch (error) {
        console.error('Failed to save to localStorage:', error);
        return false;
    }
}
```

#### `loadFromLocalStorage()`
Loads config from localStorage.

```javascript
function loadFromLocalStorage() {
    try {
        const stored = localStorage.getItem('ccm_config');
        return stored ? JSON.parse(stored) : null;
    } catch (error) {
        console.error('Failed to load from localStorage:', error);
        return null;
    }
}
```

#### `loadConfig()` - Called once on page load
Fetches config from server and saves to localStorage and appState.

```javascript
async function loadConfig() {
    try {
        const response = await fetch('/api/config/json');
        const config = await response.json();
        appState.config = config;
        appState.loaded = true;
        saveToLocalStorage(config);
        return config;
    } catch (error) {
        console.error('Failed to load config:', error);
        notifyError('Failed to load configuration');
        return null;
    }
}
```

#### `syncToServer()` - Called only from save buttons
Sends localStorage config to server.

```javascript
async function syncToServer() {
    try {
        const response = await fetch('/api/config/json', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(appState.config)
        });
        if (response.ok) {
            saveToLocalStorage(appState.config);
        }
        return response.ok;
    } catch (error) {
        console.error('Failed to sync to server:', error);
        return false;
    }
}
```

### Initialization

```javascript
window.addEventListener('DOMContentLoaded', async () => {
    await loadConfig();  // Fetch from server once
    handleRoute();
    renderOverview();
    updateLastSaved();
});
```

## Usage Examples

### Delete Provider

```javascript
async function deleteProvider(index) {
    if (!confirm('Are you sure you want to delete this Provider?')) {
        return;
    }

    try {
        // 1. Delete from state
        appState.config.providers.splice(index, 1);

        // 2. Save to localStorage only (NOT server)
        saveToLocalStorage(appState.config);

        // 3. Update UI immediately
        renderProvidersList();
        renderOverview();

        // 4. Notify save required
        notifySuccess('Provider deleted (click Save to apply)');
    } catch (error) {
        console.error('Failed to delete provider:', error);
        notifyError('Failed to delete Provider');
    }
}
```

### Add Model

```javascript
document.getElementById('add-model-form').addEventListener('submit', async function(e) {
    e.preventDefault();

    const newModel = {
        // ... collect form data
    };

    try {
        // 1. Add to state
        if (!appState.config.models) {
            appState.config.models = [];
        }
        appState.config.models.push(newModel);

        // 2. Save to localStorage only (NOT server)
        saveToLocalStorage(appState.config);

        // 3. Reset form and navigate
        notifySuccess('Model added (click Save to apply)');
        e.target.reset();
        navigate({ tab: 'models', view: null });
    } catch (error) {
        console.error('Failed to add model:', error);
        notifyError('Error adding model');
    }
});
```

### Save All

```javascript
async function saveAllConfig() {
    console.log('Saving all configuration...');

    try {
        // Sync localStorage → server
        const success = await syncToServer();

        if (success) {
            updateLastSaved();
            notifySuccess('All settings saved');
            renderOverview();
        } else {
            notifyError('Save failed');
        }
    } catch (error) {
        console.error('Failed to save all config:', error);
        notifyError('Error during save');
    }
}
```

### Save & Restart

```javascript
async function saveAndRestart() {
    if (!confirm('Save settings and restart server?')) return;

    try {
        // 1. Save first
        await saveAllConfig();

        // 2. Wait briefly then restart
        setTimeout(async () => {
            await fetch('/api/restart', { method: 'POST' });
            notifySuccess('Server restarted');
        }, 500);
    } catch (error) {
        console.error('Failed to save and restart:', error);
        notifyError('Failed to save and restart');
    }
}
```

## Data Flow

### Page Load
```
User accesses /admin
    ↓
loadConfig() called
    ↓
GET /api/config/json
    ↓
appState.config = response
    ↓
saveToLocalStorage(config)
    ↓
renderOverview()
```

### Add Provider/Model
```
User submits add form
    ↓
appState.config.providers.push(newProvider)
    ↓
saveToLocalStorage(appState.config)
    ↓
navigate({ tab: 'providers', view: null })
    ↓
renderProvidersList() (reads from localStorage)
```

### Delete Provider/Model
```
User clicks delete button
    ↓
appState.config.providers.splice(index, 1)
    ↓
saveToLocalStorage(appState.config)
    ↓
renderProvidersList() (reads from localStorage)
```

### Save Button Click
```
User clicks "Save All"
    ↓
syncToServer() called
    ↓
POST /api/config/json (appState.config)
    ↓
Server saves to TOML file
    ↓
notifySuccess('Saved')
```

### Save & Restart Click
```
User clicks "Save & Restart"
    ↓
saveAllConfig() → syncToServer()
    ↓
POST /api/config/json
    ↓
POST /api/restart
    ↓
Server restarts → loads new config
```

## Benefits

### 1. Solves server restart problem
```javascript
// ❌ Before: Server returns stale data
appState.config.providers.push(newProvider);
await saveToServer();  // Saves to TOML
const config = await fetch('/api/config/json');  // ⚠️ Stale data!

// ✅ After: Immediate reflection via localStorage
appState.config.providers.push(newProvider);
saveToLocalStorage(appState.config);  // Immediate reflection!
renderProvidersList();  // Reads from localStorage
```

### 2. Performance improvement
- Eliminates unnecessary network requests
- Immediate UI updates
- Editable config even offline

### 3. Better user experience
- All changes immediately reflected in UI
- Explicit save prevents unintended changes
- "Unsaved changes" state clearly indicated

### 4. Data consistency
- localStorage is single source of truth
- Server sync only on explicit save
- Changes persist across page refreshes

## Caveats

### 1. Server sync required
Users must click "Save" button. Otherwise:
- Changes only in localStorage
- Changes lost on server restart
- Changes not visible from other browsers/devices

### 2. localStorage capacity limits
- Most browsers: 5-10MB limit
- CCM config typically tens of KB, so not a problem

### 3. Multi-tab sync
Current implementation assumes single tab. With simultaneous edits in multiple tabs:
- Each tab uses independent localStorage
- Last save overwrites previous save
- Future improvement: Sync tabs via `storage` event

### 4. Page refresh
```javascript
// localStorage priority on page refresh
window.addEventListener('DOMContentLoaded', async () => {
    // Re-fetch from server even with localStorage data
    // Reason: Another user may have saved changes
    await loadConfig();  // Server fetch → update localStorage
    renderOverview();
});
```

### 5. Error handling
```javascript
try {
    saveToLocalStorage(appState.config);
} catch (error) {
    // QuotaExceededError, SecurityError, etc.
    console.error('Failed to save to localStorage:', error);
    notifyError('Failed to save locally');
}
```

## Best Practices

### 1. Always show notifications
```javascript
notifySuccess('Provider added (click Save to apply)');
```
Clearly inform users that save is required.

### 2. Maintain state consistency
```javascript
// State update → localStorage save → UI update
appState.config.providers.push(newProvider);
saveToLocalStorage(appState.config);
renderProvidersList();
```

### 3. Restore on error
```javascript
const backup = JSON.parse(JSON.stringify(appState.config));
try {
    appState.config.providers.splice(index, 1);
    saveToLocalStorage(appState.config);
    renderProvidersList();
} catch (error) {
    appState.config = backup;  // Restore
    notifyError('Operation failed');
}
```

### 4. Confirm server sync
```javascript
async function syncToServer() {
    const response = await fetch('/api/config/json', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(appState.config)
    });

    if (response.ok) {
        // Only update localStorage on success
        saveToLocalStorage(appState.config);
    }

    return response.ok;
}
```

## File Location

Implementation is in:
- `src/server/admin.html` (lines 592-647, 802-827, 939-964, 1218-1273, 1319-1337)

## Related Documentation

- [URL-based State Management](./url-state-management.md) - Navigation state management using URL parameters

## Future Improvements

1. **Change tracking**
   - Compare localStorage vs server state
   - Add "unsaved changes" indicator

2. **Auto-save**
   - Option: Auto-save every N seconds
   - Warn on page exit

3. **Multi-tab sync**
   ```javascript
   window.addEventListener('storage', (e) => {
       if (e.key === 'ccm_config') {
           appState.config = JSON.parse(e.newValue);
           renderOverview();
       }
   });
   ```

4. **Undo/Redo**
   - Save change history
   - Support Ctrl+Z / Ctrl+Shift+Z

5. **Optimistic UI updates**
   - Update UI before server response
   - Rollback on failure
