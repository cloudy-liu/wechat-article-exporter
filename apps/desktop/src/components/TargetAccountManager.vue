<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

type TargetAccount = {
  fakeid: string;
  nickname: string;
  alias: string;
  round_head_img: string;
  service_type: number;
  signature: string;
};

type TargetAccountExport = {
  format: string;
  accounts: TargetAccount[];
};

type TargetAccountSearchResponse = {
  total: number;
  list: TargetAccount[];
};

type ArticleListSyncRecord = {
  target_account_id: string;
  requested_limit: number;
  fetched_count: number;
  total_count?: number | null;
  status: 'running' | 'completed' | 'failed';
  error_message?: string | null;
};

type DesktopSettings = {
  syncDownload?: {
    pageSize?: number;
  };
};

const keyword = ref('');
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);
const isSearchOpen = ref(false);
const searchResults = ref<TargetAccount[]>([]);
const accounts = ref<TargetAccount[]>([]);
const selectedAccountIds = ref<string[]>([]);
const pageSize = ref(20);
const importFileRef = ref<HTMLInputElement | null>(null);
const failedAvatarIds = ref<string[]>([]);

const selectedAccounts = computed(() =>
  accounts.value.filter(account => selectedAccountIds.value.includes(account.fakeid)),
);

const allVisibleAccountsSelected = computed(() =>
  accounts.value.length > 0 && accounts.value.every(account => selectedAccountIds.value.includes(account.fakeid)),
);

onMounted(async () => {
  await loadDesktopSettings();
  await refreshAccounts();
});

async function loadDesktopSettings() {
  try {
    const settings = await invoke<DesktopSettings>('load_desktop_settings');
    pageSize.value = positiveInteger(settings.syncDownload?.pageSize, pageSize.value);
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

function toggleSearchPanel() {
  isSearchOpen.value = !isSearchOpen.value;
  errorMessage.value = '';
}

async function searchAccounts() {
  const trimmedKeyword = keyword.value.trim();
  if (!trimmedKeyword) {
    errorMessage.value = '请输入目标公众号关键词。';
    return;
  }

  isBusy.value = true;
  errorMessage.value = '';
  message.value = '正在搜索目标公众号';

  try {
    const response = await invoke<TargetAccountSearchResponse>('search_target_accounts', {
      keyword: trimmedKeyword,
      begin: 0,
      count: 10,
    });
    searchResults.value = response.list;
    message.value = `找到 ${response.total} 个结果`;
  } catch (error) {
    errorMessage.value = formatError(error);
    message.value = '搜索需要先完成公众号平台登录';
  } finally {
    isBusy.value = false;
  }
}

async function addAccount(account: TargetAccount) {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke('add_target_account', { account });
    await refreshAccounts();
    selectedAccountIds.value = Array.from(new Set([...selectedAccountIds.value, account.fakeid]));
    message.value = `已添加 ${account.nickname} 到本地归档`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function refreshAccounts() {
  try {
    accounts.value = await invoke<TargetAccount[]>('list_target_accounts');
    selectedAccountIds.value = selectedAccountIds.value.filter(fakeid =>
      accounts.value.some(account => account.fakeid === fakeid),
    );
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

function toggleAccountSelection(account: TargetAccount) {
  if (selectedAccountIds.value.includes(account.fakeid)) {
    selectedAccountIds.value = selectedAccountIds.value.filter(fakeid => fakeid !== account.fakeid);
  } else {
    selectedAccountIds.value = [...selectedAccountIds.value, account.fakeid];
  }
}

function toggleAllAccounts() {
  if (allVisibleAccountsSelected.value) {
    selectedAccountIds.value = [];
  } else {
    selectedAccountIds.value = accounts.value.map(account => account.fakeid);
  }
}

async function deleteSelectedAccounts() {
  if (selectedAccounts.value.length === 0) {
    errorMessage.value = '请先选择要删除的公众号。';
    return;
  }

  isBusy.value = true;
  errorMessage.value = '';

  try {
    for (const account of selectedAccounts.value) {
      await invoke('delete_target_account', { fakeid: account.fakeid });
    }
    const deletedCount = selectedAccounts.value.length;
    selectedAccountIds.value = [];
    await refreshAccounts();
    message.value = `已删除 ${deletedCount} 个公众号`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function deleteAccount(account: TargetAccount) {
  selectedAccountIds.value = [account.fakeid];
  await deleteSelectedAccounts();
}

async function syncSelectedArticles() {
  if (selectedAccounts.value.length === 0) {
    errorMessage.value = '请先选择要同步的公众号。';
    return;
  }

  for (const account of selectedAccounts.value) {
    await syncArticles(account);
  }
}

async function syncArticles(account: TargetAccount) {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = `正在同步 ${account.nickname} 的文章`;

  try {
    const syncStatus = await invoke<ArticleListSyncRecord>('sync_target_account_articles', {
      fakeid: account.fakeid,
      pageSize: pageSize.value,
    });
    message.value = `已同步 ${account.nickname} 的 ${syncStatus.fetched_count} 篇文章`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}。可以重试同步。`;
  } finally {
    isBusy.value = false;
  }
}

async function exportAccounts() {
  if (selectedAccounts.value.length === 0) {
    errorMessage.value = '请先选择要导出的公众号。';
    return;
  }

  isBusy.value = true;
  errorMessage.value = '';

  try {
    const exported = await invoke<TargetAccountExport>('export_target_accounts');
    const selectedIds = new Set(selectedAccountIds.value);
    const selectedExport: TargetAccountExport = {
      format: exported.format,
      accounts: exported.accounts.filter(account => selectedIds.has(account.fakeid)),
    };
    downloadJson(selectedExport, '公众号.json');
    message.value = `已导出 ${selectedExport.accounts.length} 个公众号`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

function openImportFilePicker() {
  importFileRef.value?.click();
}

async function importAccounts(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) {
    return;
  }

  isBusy.value = true;
  errorMessage.value = '';

  try {
    const parsed = JSON.parse(await file.text()) as TargetAccountExport;
    await invoke('import_target_accounts', { export: parsed });
    await refreshAccounts();
    message.value = `目标公众号已导入`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
    if (importFileRef.value) {
      importFileRef.value.value = '';
    }
  }
}

function downloadJson(exported: TargetAccountExport, filename: string) {
  const blob = new Blob([JSON.stringify(exported, null, 2)], { type: 'application/json;charset=utf-8' });
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  window.URL.revokeObjectURL(url);
}

function isSelected(account: TargetAccount): boolean {
  return selectedAccountIds.value.includes(account.fakeid);
}

function accountAvatarSrc(account: TargetAccount): string {
  if (failedAvatarIds.value.includes(account.fakeid)) {
    return '';
  }

  return normalizeWechatImageUrl(account.round_head_img);
}

function normalizeWechatImageUrl(value: string): string {
  const url = value.trim();
  if (!url) {
    return '';
  }
  if (url.startsWith('//')) {
    return `https:${url}`;
  }
  if (url.startsWith('http://')) {
    return `https://${url.slice('http://'.length)}`;
  }
  if (url.startsWith('https://')) {
    return url;
  }
  if (url.startsWith('/')) {
    return `https://mp.weixin.qq.com${url}`;
  }

  return url;
}

function markAvatarFailed(account: TargetAccount) {
  if (!failedAvatarIds.value.includes(account.fakeid)) {
    failedAvatarIds.value = [...failedAvatarIds.value, account.fakeid];
  }
}

function accountAvatarFallback(account: TargetAccount): string {
  return (account.nickname || account.alias || '号').trim().slice(0, 1);
}

function displayAlias(account: TargetAccount): string {
  return account.alias || '未设置';
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function positiveInteger(value: unknown, fallback: number): number {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number) && number > 0 ? Math.floor(number) : fallback;
}
</script>

<template>
  <section class="target-account-manager desktop-data-page" aria-labelledby="target-account-manager-title">
    <header class="manager-toolbar desktop-page-toolbar">
      <h3 id="target-account-manager-title" class="visually-hidden">搜索和管理公众号</h3>
      <div class="desktop-bulk-toolbar desktop-bulk-toolbar--leading">
        <button type="button" class="primary-button" :disabled="isBusy" @click="toggleSearchPanel">
          {{ isSearchOpen ? '收起添加' : '添加' }}
        </button>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="openImportFilePicker">
          批量导入
        </button>
        <input
          ref="importFileRef"
          type="file"
          accept=".json,application/json"
          class="hidden-file-input"
          @change="importAccounts"
        />
        <button
          type="button"
          class="secondary-button"
          :disabled="isBusy || selectedAccounts.length === 0"
          @click="exportAccounts"
        >
          批量导出
        </button>
        <button
          type="button"
          class="secondary-button"
          :disabled="isBusy || selectedAccounts.length === 0"
          @click="deleteSelectedAccounts"
        >
          删除
        </button>
        <button
          type="button"
          class="secondary-button"
          :disabled="isBusy || selectedAccounts.length === 0"
          @click="syncSelectedArticles"
        >
          同步
        </button>
      </div>
    </header>

    <section v-if="isSearchOpen" class="account-add-panel desktop-table-shell" aria-label="添加目标公众号">
      <form class="search-row desktop-control-group" @submit.prevent="searchAccounts">
        <label class="desktop-field">
          <span>搜索</span>
          <input
            v-model="keyword"
            type="search"
            placeholder="公众号名称或关键词"
            autocomplete="off"
          />
        </label>
        <button type="submit" class="primary-button" :disabled="isBusy">搜索</button>
      </form>

      <div
        v-if="searchResults.length"
        class="account-data-table account-data-table--search"
        role="table"
        aria-label="搜索结果"
      >
        <div class="account-data-table__head" role="row">
          <span role="columnheader">头像</span>
          <span role="columnheader">公众号</span>
          <span role="columnheader">简介</span>
          <span role="columnheader">操作</span>
        </div>
        <article
          v-for="account in searchResults"
          :key="account.fakeid"
          class="account-data-table__row account-table-row"
          role="row"
        >
          <span class="account-avatar-cell">
            <img v-if="accountAvatarSrc(account)" :src="accountAvatarSrc(account)" alt="" @error="markAvatarFailed(account)" />
            <span v-else class="account-avatar-fallback">{{ accountAvatarFallback(account) }}</span>
          </span>
          <div class="article-cell">
            <strong>{{ account.nickname }}</strong>
            <span>微信号：{{ displayAlias(account) }}</span>
          </div>
          <p class="account-description">{{ account.signature || '--' }}</p>
          <div class="account-row-actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="addAccount(account)">
              添加
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">输入关键词后搜索要采集的目标公众号。</p>
    </section>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <main class="desktop-table-shell account-table-shell" aria-label="本地归档公众号">
      <div
        v-if="accounts.length"
        class="account-data-table account-data-table--accounts"
        role="table"
        aria-label="本地归档"
      >
        <div class="account-data-table__head" role="row">
          <label class="row-check">
            <input type="checkbox" :checked="allVisibleAccountsSelected" @change="toggleAllAccounts" />
            <span>选择</span>
          </label>
          <span role="columnheader">头像</span>
          <span role="columnheader">名称</span>
          <span role="columnheader">简介</span>
          <span role="columnheader">操作</span>
        </div>
        <article
          v-for="account in accounts"
          :key="account.fakeid"
          class="account-data-table__row account-table-row"
          role="row"
        >
          <label class="row-check">
            <input type="checkbox" :checked="isSelected(account)" @change="toggleAccountSelection(account)" />
            <span>{{ isSelected(account) ? '已选' : '选择' }}</span>
          </label>
          <span class="account-avatar-cell">
            <img v-if="accountAvatarSrc(account)" :src="accountAvatarSrc(account)" alt="" @error="markAvatarFailed(account)" />
            <span v-else class="account-avatar-fallback">{{ accountAvatarFallback(account) }}</span>
          </span>
          <div class="article-cell">
            <strong>{{ account.nickname || account.fakeid }}</strong>
            <span>微信号：{{ displayAlias(account) }}</span>
          </div>
          <p class="account-description">{{ account.signature || '--' }}</p>
          <div class="account-row-actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="syncArticles(account)">
              同步
            </button>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="deleteAccount(account)">
              删除
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">还没有保存目标公众号。点击添加，搜索并保存要采集的公众号。</p>
    </main>
  </section>
</template>
