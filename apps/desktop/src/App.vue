<script setup lang="ts">
import { computed } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { routes } from './router';

const route = useRoute();

const navItems = computed(() => routes.filter(item => item.path !== '/' && item.name));
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <p class="brand-kicker">Desktop Archive App</p>
        <h1>WeChat Article Exporter</h1>
      </div>

      <nav class="navigation" aria-label="Desktop core feature set">
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
    </aside>

    <main class="workspace">
      <header class="workspace-header">
        <p class="workspace-kicker">Tauri 2 Shell</p>
        <h2>{{ String(route.name || 'Target Accounts') }}</h2>
      </header>

      <RouterView />
    </main>
  </div>
</template>
