# Desktop UI Components

A dedicated UI component library for desktop apps, for building modern desktop-app interfaces.

## Install / Import

```javascript
// Import everything
import { DesktopWindow, TitleBar, DesktopSidebar, DesktopCard, DesktopGrid } from './components'

// Or import individually
import DesktopWindow from './components/DesktopWindow.vue'
```

## Component List

### 1. DesktopWindow — desktop window container

The main container component for a desktop app. Provides window structure, background effects and layout management.

**Props:**
- `title` (String): Window title
- `icon` (String): Window icon path
- `showTitleBar` (Boolean): Whether to show the title bar. Default: `true`
- `hasSidebar` (Boolean): Whether a sidebar is present. Default: `false`
- `theme` (String): Theme style — `dark` or `light`. Default: `dark`

**Slots:**
- `titlebar`: Custom title bar
- `sidebar`: Sidebar content
- `default`: Main content area
- `footer`: Footer content

**Example:**
```vue
<template>
  <DesktopWindow title="My App" :has-sidebar="true">
    <template #sidebar>
      <DesktopSidebar :items="navItems" />
    </template>
    
    <template #default>
      <div>Main content</div>
    </template>
  </DesktopWindow>
</template>
```

---

### 2. TitleBar — title bar component

Custom window title bar. Supports dragging, icons and window control buttons.

**Props:**
- `title` (String): Title text
- `icon` (String): Icon path
- `draggable` (Boolean): Whether draggable. Default: `true`
- `showControls` (Boolean): Whether to show window control buttons. Default: `true`

**Slots:**
- `left`: Left-side content
- `center`: Center content
- `right`: Right-side content

**Example:**
```vue
<TitleBar title="App Title" icon="/icon.png">
  <template #left>
    <span>Custom left-side content</span>
  </template>
</TitleBar>
```

---

### 3. WindowControls — window control buttons

Minimize / maximize / close buttons for the window.

**Props:**
- `disabled` (Boolean): Whether disabled. Default: `false`

**Events:**
- `minimize`: Minimize clicked
- `maximize`: Maximize clicked
- `close`: Close clicked

**Example:**
```vue
<WindowControls @close="handleClose" />
```

---

### 4. DesktopSidebar — sidebar navigation

Sidebar navigation for desktop apps. Supports icons, labels, badges, etc.

**Props:**
- `items` (Array): Nav item array
  - `id` (String): Unique ID
  - `label` (String): Label text
  - `icon` (Component): Icon component
  - `badge` (String): Badge text (optional)
  - `disabled` (Boolean): Whether disabled (optional)
- `bottomItems` (Array): Bottom nav items (same format as items)
- `activeItem` (String): ID of the currently active item
- `collapsed` (Boolean): Whether collapsed. Default: `false`
- `collapsible` (Boolean): Whether collapsible. Default: `false`
- `showDivider` (Boolean): Whether to show a divider. Default: `true`

**Events:**
- `item-click`: Fired when a nav item is clicked
- `update:activeItem`: Fired when the active item changes
- `update:collapsed`: Fired when collapsed state changes

**Example:**
```vue
<DesktopSidebar
  :items="navItems"
  :bottom-items="bottomItems"
  :active-item="activeNav"
  @item-click="handleNavClick"
/>
```

**Nav-item format:**
```javascript
const navItems = [
  {
    id: 'dashboard',
    label: 'Dashboard',
    icon: IconDashboard,
    badge: '3', // optional
    disabled: false // optional
  }
]
```

---

### 5. DesktopCard — desktop card

Card container component for desktop apps. Supports multiple styles and interactions.

**Props:**
- `title` (String): Card title
- `icon` (Component): Title icon
- `variant` (String): Style variant — `default` | `primary` | `secondary` | `success` | `warning` | `danger`. Default: `default`
- `hoverable` (Boolean): Show hover effect. Default: `false`
- `clickable` (Boolean): Whether clickable. Default: `false`
- `showHeader` (Boolean): Whether to show the header. Default: `true`
- `noPadding` (Boolean): Remove inner padding. Default: `false`

**Slots:**
- `title`: Custom title (overrides the title prop)
- `actions`: Header action buttons
- `default`: Card content
- `footer`: Card footer

**Events:**
- `click`: Fired when the card is clicked (requires `clickable` = `true`)

**Example:**
```vue
<DesktopCard 
  title="System Info" 
  :icon="IconInfo"
  variant="primary"
  hoverable
>
  <template #actions>
    <button>Action</button>
  </template>
  
  <div>Card content</div>
  
  <template #footer>
    <button>OK</button>
  </template>
</DesktopCard>
```

---

### 6. DesktopGrid — grid layout

Responsive grid layout component for arranging cards or other elements.

**Props:**
- `cols` (Number): Column count (1–6). Default: `2`
- `gap` (String): Spacing — `xs` | `sm` | `md` | `lg` | `xl`. Default: `md`
- `responsive` (Boolean): Whether responsive. Default: `true`

**Example:**
```vue
<DesktopGrid cols="4" gap="lg">
  <DesktopCard v-for="item in items" :key="item.id">
    {{ item.content }}
  </DesktopCard>
</DesktopGrid>
```

---

## Composables

### useWindowControls

Window-control composable — provides window operations.

**Return value:**
```javascript
{
  tauriWindow,      // Tauri window object
  isMaximized,      // Whether maximized
  isMinimized,      // Whether minimized
  isFocused,        // Whether focused
  minimize,         // Minimize function
  maximize,         // Maximize function
  unmaximize,       // Restore function
  toggleMaximize,   // Toggle maximize state
  close,            // Close window
  show,             // Show window
  hide,             // Hide window
  setFocus,         // Focus window
  center,           // Center window
  setSize,          // Set window size
  getSize           // Get window size
}
```

**Example:**
```vue
<script setup>
import { useWindowControls } from '../composables'

const { isMaximized, minimize, maximize, close } = useWindowControls()
</script>
```

---

## Complete Usage Example

```vue
<template>
  <DesktopWindow title="My Desktop App" :has-sidebar="true">
    <template #sidebar>
      <DesktopSidebar
        :items="navItems"
        :active-item="activeNav"
        @item-click="handleNavClick"
      />
    </template>

    <template #default>
      <div class="page-container">
        <DesktopGrid cols="3" gap="md">
          <DesktopCard 
            v-for="card in cards" 
            :key="card.id"
            :title="card.title"
            :variant="card.variant"
            hoverable
          >
            {{ card.content }}
          </DesktopCard>
        </DesktopGrid>
      </div>
    </template>
  </DesktopWindow>
</template>

<script setup>
import { ref } from 'vue'
import { DesktopWindow, DesktopSidebar, DesktopCard, DesktopGrid } from './components'

const activeNav = ref('dashboard')
const navItems = [/* ... */]
const cards = [/* ... */]

function handleNavClick(item) {
  activeNav.value = item.id
}
</script>
```

---

## Styling and Theming

All components support theming via CSS variables:

```less
:root {
  --desktop-bg-primary: #0f0f23;
  --desktop-bg-secondary: #1a1a2e;
  --desktop-accent-cyan: #00fff5;
  --desktop-border-color: rgba(0, 255, 245, 0.2);
}
```

---

## Best Practices

1. **Use DesktopWindow as the root container** — provides the full desktop-app layout.
2. **Compose components** — DesktopSidebar + DesktopCard + DesktopGrid.
3. **Use the composables** — `useWindowControls` for window-state management.
4. **Responsive design** — take advantage of DesktopGrid's responsive behavior.
5. **Theme consistency** — use the same variant scheme and theme variables across the app.

---

## Notes

- Components use `-webkit-app-region: drag` to enable window dragging; make sure to define `no-drag` regions where needed.
- Window-control features only work inside a Tauri environment; browser contexts degrade gracefully.
- Some components rely on specific CSS variables — make sure the relevant style files are imported.
