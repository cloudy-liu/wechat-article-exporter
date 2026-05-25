<script setup lang="ts">
import { computed } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import OfficialAccountLoginPanel from './components/OfficialAccountLoginPanel.vue';
import { routes } from './router';

const route = useRoute();

const navItems = computed(() => routes.filter(item => item.path !== '/' && item.name));
</script>

<template>
  <div class="legacy-dashboard-shell app-shell">
    <aside class="legacy-sidebar sidebar">
      <div class="brand">
        <p class="brand-kicker">本地桌面端</p>
        <h1>公众号文章导出工具</h1>
      </div>

      <nav class="navigation" aria-label="桌面端核心功能">
        <RouterLink
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="nav-link"
          :class="{ active: route.path === item.path }"
        >
          <span>{{ item.name }}</span>
        </RouterLink>
      </nav>
      <OfficialAccountLoginPanel />
    </aside>

    <main class="workspace">
      <header class="legacy-topbar workspace-header">
        <h2>{{ String(route.name || '公众号管理') }}</h2>
        <div id="topbar-actions" class="legacy-topbar-actions" />
      </header>

      <RouterView />
    </main>
  </div>
</template>
