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
    </aside>

    <main class="workspace">
      <header class="workspace-header">
        <p class="workspace-kicker">公众号作者工作台</p>
        <h2>{{ String(route.name || '目标公众号') }}</h2>
      </header>

      <RouterView />
    </main>
  </div>
</template>
