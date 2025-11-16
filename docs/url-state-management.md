# URL-based State Management

The admin UI uses URLs as the single source of truth for state management.

## Concept

URL-based state management is a pattern that encodes application state in URL parameters. This pattern provides the following benefits:

- **Shareable URLs**: Specific view states can be shared via URL
- **Browser history integration**: Back/forward buttons work naturally
- **Persistence across page refreshes**: State persists after refresh
- **Bookmarkable**: Specific views can be bookmarked
- **Simple state management**: No need for complex state management libraries

## Reference

This implementation follows the pattern described in:
https://alfy.blog/2025/10/31/your-url-is-your-state.html

## Implementation

### URL Parameter Structure

```
?tab=<tab-name>&view=<view-name>
```

- `tab`: Currently active tab (overview, models, providers, router, settings)
- `view`: View state within the tab (add, edit, etc.; defaults to list view if absent)

#### Examples

```
# Overview tab
http://localhost:13456/admin

# Provider list
http://localhost:13456/admin?tab=providers

# Provider add view
http://localhost:13456/admin?tab=providers&view=add

# Model list
http://localhost:13456/admin?tab=models

# Model add view
http://localhost:13456/admin?tab=models&view=add
```

### Core Functions

#### `getURLParams()`
Parses the current URL's search parameters.

```javascript
function getURLParams() {
    return new URLSearchParams(window.location.search);
}
```

#### `updateURL(params, replace)`
Updates the URL and triggers routing.

```javascript
function updateURL(params, replace = false) {
    const url = new URL(window.location);
    Object.entries(params).forEach(([key, value]) => {
        if (value === null || value === undefined) {
            url.searchParams.delete(key);
        } else {
            url.searchParams.set(key, value);
        }
    });

    if (replace) {
        window.history.replaceState({}, '', url);
    } else {
        window.history.pushState({}, '', url);
    }

    handleRoute();
}
```

#### `navigate(params)`
Navigates with a new history entry.

```javascript
function navigate(params) {
    updateURL(params, false);
}
```

#### `handleRoute()`
Reads the URL and updates the UI accordingly.

```javascript
function handleRoute() {
    const params = getURLParams();
    const tab = params.get('tab') || 'overview';
    const view = params.get('view');

    // Hide all tabs
    document.querySelectorAll('.tab-content').forEach(el => el.classList.add('hidden'));
    document.querySelectorAll('[id^="tab-"]').forEach(el => {
        el.classList.remove('tab-active');
        el.classList.add('text-gray-600');
    });

    // Show selected tab
    document.getElementById('content-' + tab).classList.remove('hidden');
    const tabBtn = document.getElementById('tab-' + tab);
    if (tabBtn) {
        tabBtn.classList.add('tab-active');
        tabBtn.classList.remove('text-gray-600');
    }

    // Handle views based on tab and view parameters
    if (tab === 'providers') {
        if (view === 'add') {
            // Show add provider view
        } else {
            // Show provider list
        }
    } else if (tab === 'models') {
        if (view === 'add') {
            // Show add model view
        } else {
            // Show model list
        }
    }
}
```

### Usage Examples

#### Tab switching

```javascript
function showTab(tabName) {
    navigate({ tab: tabName, view: null });
}
```

#### View switching

```javascript
function showAddProvider() {
    navigate({ tab: 'providers', view: 'add' });
}

function showProvidersList() {
    navigate({ tab: 'providers', view: null });
}
```

#### Navigation after form submit

```javascript
document.getElementById('add-provider-form').addEventListener('submit', async function(e) {
    e.preventDefault();

    // ... API call ...

    if (saveResponse.ok) {
        notifySuccess('Provider added');
        e.target.reset();
        navigate({ tab: 'providers', view: null });  // Return to list
        loadOverview();
    }
});
```

### Event Listeners

#### Route on page load

```javascript
window.addEventListener('DOMContentLoaded', () => {
    handleRoute();  // Set initial state based on URL
    loadOverview();
    updateLastSaved();
});
```

#### Browser history support

```javascript
// Support back/forward buttons
window.addEventListener('popstate', handleRoute);
```

## Benefits

### 1. Declarative navigation
```javascript
// Before: Manual DOM manipulation
document.getElementById('providers-list-view').classList.remove('hidden');
document.getElementById('providers-add-view').classList.add('hidden');

// After: Declarative navigation
navigate({ tab: 'providers', view: null });
```

### 2. Automatic history management
Browser's back/forward buttons work automatically.

### 3. Shareable deep links
```
http://localhost:13456/admin?tab=providers&view=add
```
Sharing this URL takes the recipient directly to the Provider add screen.

### 4. Easy testing
Direct access to specific states via URL makes testing easier.

## Best Practices

### 1. Clean up parameters
Remove unnecessary parameters by setting them to `null` or `undefined`.

```javascript
navigate({ tab: 'providers', view: null });  // Remove view parameter
```

### 2. Handle defaults
Use default values when parameters are missing from URL.

```javascript
const tab = params.get('tab') || 'overview';  // Default: overview
```

### 3. Separate from data loading
Separate view switching from data loading.

```javascript
function handleRoute() {
    // ...
    if (view === 'add') {
        showAddView();
        loadAddViewData();  // Load data in separate function
    }
}
```

### 4. Replace vs Push
- **Push**: Normal navigation (adds to history)
- **Replace**: Replace current history (redirects, etc.)

```javascript
navigate({ tab: 'providers' });           // pushState
updateURL({ tab: 'providers' }, true);    // replaceState
```

## Caveats

### 1. Initialization order
Call `handleRoute()` before other initialization on page load.

```javascript
window.addEventListener('DOMContentLoaded', () => {
    handleRoute();      // First set state based on URL
    loadOverview();     // Then load data
    updateLastSaved();
});
```

### 2. Prevent infinite loops
Don't call `navigate()` inside `handleRoute()`.

```javascript
// ❌ Bad
function handleRoute() {
    navigate({ tab: 'overview' });  // Infinite loop!
}

// ✅ Good
function handleRoute() {
    // Only read URL and update UI
}
```

### 3. Sensitive data
Don't include sensitive data in URLs. Only include IDs or state information.

```javascript
// ❌ Bad
navigate({ tab: 'providers', apiKey: 'sk-...' });

// ✅ Good
navigate({ tab: 'providers', view: 'add' });
```

## File Location

Implementation is in:
- `src/server/admin.html` (lines 586-659, 1425-1433)

## Future Improvements

- Add type safety with TypeScript
- Add URL parameter validation
- Support nested routing
- Query parameter encryption/compression (for complex state)
