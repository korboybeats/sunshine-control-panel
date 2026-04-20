# Desktop UI Framework

A modular desktop-app UI component library, built specifically for Tauri + Vue 3 desktop apps.

## 📁 Directory Structure

```
desktop/
├── components/           # Desktop UI component library
│   ├── DesktopWindow.vue    # Window container
│   ├── TitleBar.vue         # Title bar
│   ├── WindowControls.vue   # Window control buttons
│   ├── DesktopSidebar.vue   # Sidebar navigation
│   ├── DesktopCard.vue      # Card component
│   ├── DesktopGrid.vue      # Grid layout
│   ├── index.js             # Component exports
│   └── README.md            # Component docs
├── composables/          # Composition functions
│   ├── useWindowControls.js  # Window controls
│   └── index.js             # Composable exports
├── views/                # View pages
│   ├── DashboardView.vue
│   ├── DevicesView.vue
│   ├── StreamView.vue
│   ├── ToolsView.vue
│   └── SettingsView.vue
├── icons/                # Icon components
├── DesktopApp.vue        # Main app component
├── desktop.less          # Desktop styles
├── main.js               # Entry point
└── index.html            # HTML template
```

## 🚀 Quick Start

### Basic Usage

```vue
<template>
  <DesktopWindow title="My App" :has-sidebar="true">
    <template #sidebar>
      <DesktopSidebar :items="navItems" :active-item="activeNav" />
    </template>
    
    <template #default>
      <DesktopCard title="Welcome">
        Welcome to the desktop UI component library!
      </DesktopCard>
    </template>
  </DesktopWindow>
</template>

<script setup>
import { ref } from 'vue'
import { DesktopWindow, DesktopSidebar, DesktopCard } from './components'

const activeNav = ref('home')
const navItems = [
  { id: 'home', label: 'Home', icon: IconHome }
]
</script>
```

## 📦 Component List

### Core Components

| Component | Description | Docs |
|-----------|-------------|------|
| `DesktopWindow` | Desktop window container | [Docs](./components/README.md#1-desktopwindow) |
| `TitleBar` | Custom title bar | [Docs](./components/README.md#2-titlebar) |
| `WindowControls` | Window control buttons | [Docs](./components/README.md#3-windowcontrols) |
| `DesktopSidebar` | Sidebar navigation | [Docs](./components/README.md#4-desktopsidebar) |
| `DesktopCard` | Desktop card | [Docs](./components/README.md#5-desktopcard) |
| `DesktopGrid` | Grid layout | [Docs](./components/README.md#6-desktopgrid) |

### Composables

| Composable | Description | Docs |
|------------|-------------|------|
| `useWindowControls` | Window control utilities | [Docs](./components/README.md#usewindowcontrols) |

## 🎨 Features

- ✅ **Modular design** — independent, reusable, easy to maintain
- ✅ **TypeScript friendly** — full type support
- ✅ **Responsive layout** — adapts to different screen sizes
- ✅ **Theming** — supports dark / light themes
- ✅ **Tauri integration** — seamless native window control integration
- ✅ **Accessibility** — keyboard navigation and screen reader support

## 📖 Docs

For detailed component documentation, see:
- [Component docs](./components/README.md)
- [Usage examples](./components/README.md#complete-usage-examples)

## 🔧 Development

### Adding a new component

1. Create the component file under `components/`
2. Export it from `components/index.js`
3. Document it in `components/README.md`

### Adding a new Composable

1. Create the file under `composables/`
2. Export it from `composables/index.js`
3. Add usage examples

## 📝 Changelog

### v1.0.0 (current)
- ✨ Initial release
- ✨ 6 core components
- ✨ Window control composable
- ✨ Full documentation

## 🤝 Contributing

Issues and pull requests are welcome!

## 📄 License

Same as the main project license.

