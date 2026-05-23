<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

type DesktopNetworkProxySetting = {
  url: string;
  authorization?: string | null;
};

type DesktopSettings = {
  archiveDir: string;
  export: {
    markdown: boolean;
    html: boolean;
  };
  syncDownload: {
    historyLimit: number;
    pageSize: number;
    downloadConcurrency: number;
  };
  networkProxy?: DesktopNetworkProxySetting | null;
};

const defaultSettings: DesktopSettings = {
  archiveDir: '',
  export: {
    markdown: true,
    html: true,
  },
  syncDownload: {
    historyLimit: 20,
    pageSize: 5,
    downloadConcurrency: 2,
  },
  networkProxy: null,
};

const settings = ref<DesktopSettings>({ ...defaultSettings, export: { ...defaultSettings.export }, syncDownload: { ...defaultSettings.syncDownload } });
const proxyUrl = ref('');
const proxyAuthorization = ref('');
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);

const archiveDirectoryHint = computed(() =>
  settings.value.archiveDir.trim()
    ? settings.value.archiveDir
    : '首次启动后会自动使用应用数据目录下的 archive 文件夹',
);

onMounted(() => {
  loadSettings();
});

async function loadSettings() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    applySettings(await invoke<DesktopSettings>('load_desktop_settings'));
    message.value = '设置已加载';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function saveSettings() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    const saved = await invoke<DesktopSettings>('save_desktop_settings', {
      settings: buildSettingsPayload(),
    });
    applySettings(saved);
    message.value = '设置已保存';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function clearAllCredentials() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke('clear_credentials');
    message.value = '全部本地凭证已清除';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function logoutOfficialAccount() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke('logout_official_account_login');
    message.value = '已退出公众号平台登录';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

function buildSettingsPayload(): DesktopSettings {
  const proxy = proxyUrl.value.trim()
    ? {
        url: proxyUrl.value.trim(),
        authorization: proxyAuthorization.value.trim() || null,
      }
    : null;

  return {
    archiveDir: settings.value.archiveDir.trim(),
    export: {
      markdown: settings.value.export.markdown,
      html: settings.value.export.html,
    },
    syncDownload: {
      historyLimit: positiveInteger(settings.value.syncDownload.historyLimit, defaultSettings.syncDownload.historyLimit),
      pageSize: positiveInteger(settings.value.syncDownload.pageSize, defaultSettings.syncDownload.pageSize),
      downloadConcurrency: positiveInteger(
        settings.value.syncDownload.downloadConcurrency,
        defaultSettings.syncDownload.downloadConcurrency,
      ),
    },
    networkProxy: proxy,
  };
}

function applySettings(nextSettings: DesktopSettings) {
  settings.value = {
    archiveDir: nextSettings.archiveDir || '',
    export: {
      markdown: nextSettings.export?.markdown ?? true,
      html: nextSettings.export?.html ?? true,
    },
    syncDownload: {
      historyLimit: positiveInteger(nextSettings.syncDownload?.historyLimit, defaultSettings.syncDownload.historyLimit),
      pageSize: positiveInteger(nextSettings.syncDownload?.pageSize, defaultSettings.syncDownload.pageSize),
      downloadConcurrency: positiveInteger(
        nextSettings.syncDownload?.downloadConcurrency,
        defaultSettings.syncDownload.downloadConcurrency,
      ),
    },
    networkProxy: nextSettings.networkProxy || null,
  };
  proxyUrl.value = settings.value.networkProxy?.url || '';
  proxyAuthorization.value = settings.value.networkProxy?.authorization || '';
}

function positiveInteger(value: unknown, fallback: number): number {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number) && number > 0 ? Math.floor(number) : fallback;
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
</script>

<template>
  <section class="settings-workbench" aria-labelledby="settings-workbench-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">桌面端设置</p>
        <h3 id="settings-workbench-title">本地采集配置</h3>
      </div>
      <div class="settings-toolbar-actions">
        <button type="button" class="secondary-button" :disabled="isBusy" @click="loadSettings">重新加载</button>
        <button type="button" class="primary-button" :disabled="isBusy" @click="saveSettings">保存设置</button>
      </div>
    </div>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <div class="settings-grid">
      <section class="settings-panel" aria-label="本地归档">
        <div>
          <p class="section-label">本地归档</p>
          <h4>归档目录</h4>
        </div>
        <label>
          <span>本地归档位置</span>
          <input v-model="settings.archiveDir" type="text" autocomplete="off" />
        </label>
        <p class="settings-hint">{{ archiveDirectoryHint }}</p>
      </section>

      <section class="settings-panel" aria-label="导出偏好">
        <div>
          <p class="section-label">导出偏好</p>
          <h4>默认导出格式</h4>
        </div>
        <label class="settings-check">
          <input v-model="settings.export.markdown" type="checkbox" />
          <span>默认导出 Markdown</span>
        </label>
        <label class="settings-check">
          <input v-model="settings.export.html" type="checkbox" />
          <span>默认导出 HTML</span>
        </label>
      </section>

      <section class="settings-panel" aria-label="同步与下载">
        <div>
          <p class="section-label">同步与下载</p>
          <h4>采集默认值</h4>
        </div>
        <div class="settings-number-grid">
          <label>
            <span>历史文章上限</span>
            <input v-model.number="settings.syncDownload.historyLimit" type="number" min="1" max="500" />
          </label>
          <label>
            <span>同步分页数量</span>
            <input v-model.number="settings.syncDownload.pageSize" type="number" min="1" max="50" />
          </label>
          <label>
            <span>下载并发数</span>
            <input v-model.number="settings.syncDownload.downloadConcurrency" type="number" min="1" max="10" />
          </label>
        </div>
      </section>

      <section class="settings-panel" aria-label="网络代理">
        <div>
          <p class="section-label">网络代理</p>
          <h4>后端下载代理</h4>
        </div>
        <label>
          <span>代理地址</span>
          <input v-model="proxyUrl" type="url" placeholder="http://127.0.0.1:7890" autocomplete="off" />
        </label>
        <label>
          <span>代理授权</span>
          <input v-model="proxyAuthorization" type="text" placeholder="可选请求头值" autocomplete="off" />
        </label>
      </section>

      <section class="settings-panel settings-panel--danger" aria-label="清理凭证">
        <div>
          <p class="section-label">清理凭证</p>
          <h4>本地登录状态</h4>
        </div>
        <div class="settings-danger-actions">
          <button type="button" class="secondary-button" :disabled="isBusy" @click="logoutOfficialAccount">
            退出公众号登录
          </button>
          <button type="button" class="secondary-button" :disabled="isBusy" @click="clearAllCredentials">
            清除全部本地凭证
          </button>
        </div>
      </section>
    </div>
  </section>
</template>
