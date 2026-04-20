<template>
  <div class="dashboard-container">
    <h1 class="dashboard-title">Control Panel</h1>
    <div class="grid-container">
      <!-- Shortcut entry cards -->
      <a 
        v-for="(item, index) in shortcutItems" 
        :key="index" 
        class="dashboard-card"
        :href="item.path"
      >
        <div class="card-icon">
          {{ item.icon }}
        </div>
        <h3 class="card-title">{{ item.title }}</h3>
        <p class="card-desc">{{ item.description }}</p>
      </a>
    </div>
  </div>
</template>

<script setup>
import { reactive } from 'vue';

// Shortcut entry config data
const shortcutItems = reactive([
  {
    icon: '🔗',
    title: 'Pair',
    description: 'Start pairing',
    path: '/pin'
  },
  {
    icon: '🛡️',
    title: 'Security',
    description: 'Security and privacy settings',
    path: '/security.html'
  },
  {
    icon: '🌐',
    title: 'Network',
    description: 'Network connection settings',
    path: '/network.html'
  },
  {
    icon: '💾',
    title: 'Storage',
    description: 'Storage space settings',
    path: '/storage.html'
  },
  {
    icon: '🖥️',
    title: 'Display',
    description: 'Display and resolution settings',
    path: '/display.html'
  },
  {
    icon: '🔔',
    title: 'Notifications',
    description: 'Notification settings',
    path: '/notifications.html'
  },
  {
    icon: '🔌',
    title: 'Clients',
    description: 'Paired client management',
    path: '/pin'
  },
  {
    icon: '🔋',
    title: 'Power',
    description: 'Power management options',
    path: '/power-settings'
  }
]);
</script>

<style lang="less" scoped>
@dashboard-padding: 2rem;
@card-min-width: 280px;
@grid-gap: 1.5rem;
@title-color: #2c3e50;
@card-bg: white;
@border-color: #eee;

.dashboard-container {
  padding: @dashboard-padding;
  max-width: 1200px;
  margin: 0 auto;

  .dashboard-title {
    text-align: center;
    margin-bottom: 2rem;
    color: @title-color;
  }

  .grid-container {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(@card-min-width, 1fr));
    gap: @grid-gap;
    padding: 1rem;

    .dashboard-card {
      background: @card-bg;
      border-radius: 12px;
      padding: 1.5rem;
      cursor: pointer;
      transition: transform 0.2s, box-shadow 0.2s;
      border: 1px solid @border-color;

      &:hover {
        transform: translateY(-5px);
        box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
      }

      .card-icon {
        font-size: 2.5rem;
        margin-bottom: 1rem;
      }

      .card-title {
        color: #34495e;
        margin-bottom: 0.5rem;
      }

      .card-desc {
        color: #7f8c8d;
        font-size: 0.9rem;
      }
    }
  }
}
</style>
