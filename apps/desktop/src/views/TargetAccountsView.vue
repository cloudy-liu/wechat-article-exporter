<script setup lang="ts">
import OfficialAccountLoginPanel from '../components/OfficialAccountLoginPanel.vue';
import TargetAccountManager from '../components/TargetAccountManager.vue';
</script>

<template>
  <section class="target-accounts-view">
    <OfficialAccountLoginPanel />
    <TargetAccountManager />

    <section class="workflow-strip" aria-label="Target account workflow">
      <div>
        <span>1</span>
        <strong>Login</strong>
        <p>Connect the Official Account platform session.</p>
      </div>
      <div>
        <span>2</span>
        <strong>Add targets</strong>
        <p>Search and manage public Target Official Accounts.</p>
      </div>
      <div>
        <span>3</span>
        <strong>Collect</strong>
        <p>Synchronize article lists and save content locally.</p>
      </div>
    </section>
  </section>
</template>
