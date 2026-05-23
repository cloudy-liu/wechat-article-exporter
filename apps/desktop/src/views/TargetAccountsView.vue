<script setup lang="ts">
import OfficialAccountLoginPanel from '../components/OfficialAccountLoginPanel.vue';
import TargetAccountManager from '../components/TargetAccountManager.vue';
</script>

<template>
  <section class="target-accounts-view">
    <OfficialAccountLoginPanel />
    <TargetAccountManager />

    <section class="workflow-strip" aria-label="目标公众号工作流">
      <div>
        <span>1</span>
        <strong>登录</strong>
        <p>连接公众号平台运营者会话。</p>
      </div>
      <div>
        <span>2</span>
        <strong>添加目标</strong>
        <p>搜索并管理要采集的目标公众号。</p>
      </div>
      <div>
        <span>3</span>
        <strong>采集</strong>
        <p>同步文章列表，并把内容保存到本地。</p>
      </div>
    </section>
  </section>
</template>
