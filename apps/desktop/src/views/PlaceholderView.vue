<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();

const title = computed(() => String(route.meta.title || route.name || 'Desktop Workspace'));
const summary = computed(() => String(route.meta.summary || 'This workflow will be migrated from the Legacy Web App.'));
</script>

<template>
  <section class="placeholder-view">
    <div>
      <p class="section-label">Core workflow placeholder</p>
      <h3>{{ title }}</h3>
      <p>{{ summary }}</p>
    </div>
  </section>
</template>
