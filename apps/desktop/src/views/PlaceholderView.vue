<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();

const title = computed(() => String(route.meta.title || route.name || '桌面工作区'));
const summary = computed(() => String(route.meta.summary || '这个工作流会按桌面端优先级继续迁移。'));
</script>

<template>
  <section class="placeholder-view">
    <div>
      <p class="section-label">核心工作流</p>
      <h3>{{ title }}</h3>
      <p>{{ summary }}</p>
    </div>
  </section>
</template>
