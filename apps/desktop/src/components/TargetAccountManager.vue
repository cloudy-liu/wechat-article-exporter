<script setup lang="ts">
import { onMounted, ref } from 'vue';
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

type TargetArticle = {
  article_id: string;
  target_account_id: string;
  title: string;
  source_url: string;
  digest: string;
  author_name: string;
  cover: string;
  appmsgid: number;
  itemidx: number;
  item_show_type: number;
  create_time: number;
  update_time: number;
  is_deleted: boolean;
  copyright_type: number;
};

type ArticleListSyncRecord = {
  target_account_id: string;
  requested_limit: number;
  fetched_count: number;
  total_count?: number | null;
  status: 'running' | 'completed' | 'failed';
  error_message?: string | null;
};

type CollectionTaskItem = {
  task_id: string;
  item_id: string;
  item_type: string;
  status: 'waiting' | 'running' | 'cancelled' | 'succeeded' | 'failed';
  payload_json: string;
  error_message?: string | null;
  attempts: number;
};

type CollectionTask = {
  task_id: string;
  task_type: 'accountArticleSync' | 'articleHtmlDownload' | 'albumDownload' | 'export';
  target_account_id?: string | null;
  status: 'waiting' | 'running' | 'paused' | 'cancelled' | 'succeeded' | 'failed';
  total_items: number;
  waiting_items: number;
  running_items: number;
  succeeded_items: number;
  failed_items: number;
  cancelled_items: number;
  error_message?: string | null;
  items: CollectionTaskItem[];
};

type NetworkProxySetting = {
  url: string;
  authorization?: string | null;
};

type ArticleHtmlDownloadOutcome = {
  articleId: string;
  targetAccountId: string;
  sourceUrl: string;
  htmlFile: string;
  assetFiles: string[];
};

type ArticleExportOutcome = {
  articleId: string;
  sourceHtmlFile: string;
  markdownFile?: string | null;
  htmlFile?: string | null;
  taskId: string;
};

const keyword = ref('');
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);
const searchResults = ref<TargetAccount[]>([]);
const accounts = ref<TargetAccount[]>([]);
const selectedFakeid = ref('');
const articleList = ref<TargetArticle[]>([]);
const syncStatus = ref<ArticleListSyncRecord | null>(null);
const collectionTasks = ref<CollectionTask[]>([]);
const maxItems = ref(20);
const pageSize = ref(5);
const proxyUrl = ref('');
const proxyAuthorization = ref('');
const exportText = ref('');
const importText = ref('');

onMounted(() => {
  refreshAccounts();
  refreshCollectionTasks();
});

async function searchAccounts() {
  const trimmedKeyword = keyword.value.trim();
  if (!trimmedKeyword) {
    errorMessage.value = 'Enter a Target Official Account keyword.';
    return;
  }

  isBusy.value = true;
  errorMessage.value = '';
  message.value = 'Searching Target Official Accounts';

  try {
    const response = await invoke<TargetAccountSearchResponse>('search_target_accounts', {
      keyword: trimmedKeyword,
      begin: 0,
      count: 5,
    });
    searchResults.value = response.list;
    message.value = `${response.total} result${response.total === 1 ? '' : 's'} found`;
  } catch (error) {
    errorMessage.value = formatError(error);
    message.value = 'Search requires an active Official Account login';
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
    selectedFakeid.value = account.fakeid;
    await refreshArticleList(account.fakeid);
    message.value = `${account.nickname} added to the local archive`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function refreshAccounts() {
  try {
    accounts.value = await invoke<TargetAccount[]>('list_target_accounts');
    if (!selectedFakeid.value && accounts.value.length > 0) {
      selectedFakeid.value = accounts.value[0].fakeid;
      await refreshArticleList(selectedFakeid.value);
    }
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

async function deleteAccount(account: TargetAccount) {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke('delete_target_account', { fakeid: account.fakeid });
    await refreshAccounts();
    if (selectedFakeid.value === account.fakeid) {
      selectedFakeid.value = accounts.value[0]?.fakeid || '';
      articleList.value = [];
      syncStatus.value = null;
      if (selectedFakeid.value) {
        await refreshArticleList(selectedFakeid.value);
      }
    }
    message.value = `${account.nickname} deleted from the local archive`;
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function syncArticles(account: TargetAccount) {
  isBusy.value = true;
  errorMessage.value = '';
  selectedFakeid.value = account.fakeid;
  syncStatus.value = {
    target_account_id: account.fakeid,
    requested_limit: maxItems.value,
    fetched_count: 0,
    total_count: null,
    status: 'running',
    error_message: null,
  };
  message.value = `Synchronizing articles for ${account.nickname}`;

  try {
    syncStatus.value = await invoke<ArticleListSyncRecord>('sync_target_account_articles', {
      fakeid: account.fakeid,
      maxItems: maxItems.value,
      pageSize: pageSize.value,
    });
    await refreshArticleList(account.fakeid);
    await refreshCollectionTasks();
    message.value = `Synced ${syncStatus.value.fetched_count} article${syncStatus.value.fetched_count === 1 ? '' : 's'}`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}. You can retry the sync.`;
    await refreshSyncStatus(account.fakeid);
    await refreshCollectionTasks();
  } finally {
    isBusy.value = false;
  }
}

async function refreshArticleList(fakeid: string) {
  if (!fakeid) {
    articleList.value = [];
    syncStatus.value = null;
    return;
  }

  selectedFakeid.value = fakeid;
  articleList.value = await invoke<TargetArticle[]>('list_target_articles', { fakeid });
  await refreshSyncStatus(fakeid);
}

async function refreshSyncStatus(fakeid: string) {
  syncStatus.value = await invoke<ArticleListSyncRecord | null>('latest_article_list_sync', { fakeid });
}

async function refreshCollectionTasks() {
  collectionTasks.value = await invoke<CollectionTask[]>('list_collection_tasks');
}

async function downloadArticleHtml(article: TargetArticle) {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = `Downloading HTML for ${article.title || article.article_id}`;

  try {
    const outcome = await invoke<ArticleHtmlDownloadOutcome>('download_article_html', {
      request: {
        fakeid: article.target_account_id,
        articleId: article.article_id,
        proxy: networkProxySetting(),
      },
    });
    await refreshCollectionTasks();
    message.value = `Downloaded HTML to ${outcome.htmlFile}`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}. Check Collection tasks for retry details.`;
    await refreshCollectionTasks();
  } finally {
    isBusy.value = false;
  }
}

async function exportArticleArchive(article: TargetArticle, format: 'markdown' | 'html') {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = `Exporting ${format.toUpperCase()} for ${article.title || article.article_id}`;

  try {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.article_id,
        formats: [format],
      },
    });
    await refreshCollectionTasks();
    const exportedFile = format === 'markdown' ? outcome.markdownFile : outcome.htmlFile;
    message.value = `Exported ${format.toUpperCase()} to ${exportedFile || outcome.sourceHtmlFile}`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}. Download HTML before exporting this article.`;
    await refreshCollectionTasks();
  } finally {
    isBusy.value = false;
  }
}

async function retryFailedTask(task: CollectionTask) {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke<CollectionTask>('retry_failed_collection_task_items', { taskId: task.task_id });
    await refreshCollectionTasks();
    message.value = 'Failed task items are ready to retry';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function pauseTask(task: CollectionTask) {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke<CollectionTask>('pause_collection_task', { taskId: task.task_id });
    await refreshCollectionTasks();
    message.value = 'Collection task paused';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function cancelTask(task: CollectionTask) {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    await invoke<CollectionTask>('cancel_collection_task', { taskId: task.task_id });
    await refreshCollectionTasks();
    message.value = 'Collection task cancelled';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function exportAccounts() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    const exported = await invoke<TargetAccountExport>('export_target_accounts');
    exportText.value = JSON.stringify(exported, null, 2);
    message.value = 'Export JSON generated';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

async function importAccounts() {
  isBusy.value = true;
  errorMessage.value = '';

  try {
    const parsed = JSON.parse(importText.value) as TargetAccountExport;
    await invoke('import_target_accounts', { export: parsed });
    await refreshAccounts();
    message.value = 'Target Official Accounts imported';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isBusy.value = false;
  }
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function formatUnixTime(value: number): string {
  if (!value) {
    return '--';
  }

  return new Date(value * 1000).toLocaleString();
}

function formatTaskType(value: CollectionTask['task_type']): string {
  if (value === 'accountArticleSync') {
    return 'Account article sync';
  }
  if (value === 'articleHtmlDownload') {
    return 'Article HTML download';
  }
  if (value === 'export') {
    return 'Export';
  }

  return value;
}

function networkProxySetting(): NetworkProxySetting | null {
  const url = proxyUrl.value.trim();
  if (!url) {
    return null;
  }

  return {
    url,
    authorization: proxyAuthorization.value.trim() || null,
  };
}
</script>

<template>
  <section class="target-account-manager" aria-labelledby="target-account-manager-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">Target Official Accounts</p>
        <h3 id="target-account-manager-title">Search and manage accounts</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshAccounts">
        Refresh
      </button>
    </div>

    <form class="search-row" @submit.prevent="searchAccounts">
      <label>
        <span>Search</span>
        <input
          v-model="keyword"
          type="search"
          placeholder="Account name or keyword"
          autocomplete="off"
        />
      </label>
      <button type="submit" class="primary-button" :disabled="isBusy">Search</button>
    </form>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <div class="account-columns">
      <section aria-label="Search results">
        <div class="column-header">
          <strong>Search results</strong>
          <span>{{ searchResults.length }}</span>
        </div>

        <div v-if="searchResults.length" class="account-list">
          <article v-for="account in searchResults" :key="account.fakeid" class="account-row">
            <img v-if="account.round_head_img" :src="account.round_head_img" alt="" />
            <div>
              <strong>{{ account.nickname }}</strong>
              <span>{{ account.alias || account.fakeid }}</span>
              <p>{{ account.signature }}</p>
            </div>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="addAccount(account)">
              Add
            </button>
          </article>
        </div>
        <p v-else class="empty-state">No search results loaded.</p>
      </section>

      <section aria-label="Local target accounts">
        <div class="column-header">
          <strong>Local archive</strong>
          <span>{{ accounts.length }}</span>
        </div>

        <div v-if="accounts.length" class="account-list">
          <article v-for="account in accounts" :key="account.fakeid" class="account-row">
            <img v-if="account.round_head_img" :src="account.round_head_img" alt="" />
            <div>
              <strong>{{ account.nickname }}</strong>
              <span>{{ account.alias || account.fakeid }}</span>
              <p>{{ account.signature }}</p>
            </div>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="deleteAccount(account)">
              Delete
            </button>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="syncArticles(account)">
              Sync articles
            </button>
          </article>
        </div>
        <p v-else class="empty-state">No Target Official Accounts saved yet.</p>
      </section>
    </div>

    <section class="article-sync-panel" aria-label="Article list synchronization">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">Article list sync</p>
          <h3>Sync articles for a saved account</h3>
        </div>
        <button
          type="button"
          class="secondary-button"
          :disabled="isBusy || !selectedFakeid"
          @click="refreshArticleList(selectedFakeid)"
        >
          Refresh articles
        </button>
      </div>

      <div class="sync-controls">
        <label>
          <span>History limit</span>
          <input v-model.number="maxItems" type="number" min="1" max="200" />
        </label>
        <label>
          <span>Page size</span>
          <input v-model.number="pageSize" type="number" min="1" max="20" />
        </label>
      </div>

      <div class="network-proxy-controls">
        <label>
          <span>Network proxy</span>
          <input v-model="proxyUrl" type="url" placeholder="http://127.0.0.1:7890" autocomplete="off" />
        </label>
        <label>
          <span>Proxy authorization</span>
          <input v-model="proxyAuthorization" type="text" placeholder="Optional header value" autocomplete="off" />
        </label>
      </div>

      <div class="sync-summary">
        <strong>Latest sync</strong>
        <span v-if="syncStatus">
          {{ syncStatus.status }} - {{ syncStatus.fetched_count }}/{{ syncStatus.requested_limit }}
          fetched<span v-if="syncStatus.total_count"> - {{ syncStatus.total_count }} upstream</span>
        </span>
        <span v-else>No sync has run for the selected account.</span>
        <p v-if="syncStatus?.error_message">
          {{ syncStatus.error_message }}. Retry from the local archive account row.
        </p>
      </div>

      <div class="column-header">
        <strong>Synced articles</strong>
        <span>{{ articleList.length }}</span>
      </div>

      <div v-if="articleList.length" class="article-list">
        <article v-for="article in articleList" :key="article.article_id" class="article-row">
          <img v-if="article.cover" :src="article.cover" alt="" />
          <div>
            <strong>{{ article.title }}</strong>
            <span>{{ article.author_name || 'Unknown author' }} - {{ formatUnixTime(article.create_time) }}</span>
            <p>{{ article.digest }}</p>
          </div>
          <div class="article-row__actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadArticleHtml(article)">
              Download HTML
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy"
              @click="exportArticleArchive(article, 'markdown')"
            >
              Export Markdown
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy"
              @click="exportArticleArchive(article, 'html')"
            >
              Export HTML
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">No synchronized articles loaded.</p>
    </section>

    <section class="article-sync-panel" aria-label="Collection tasks">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">Collection tasks</p>
          <h3>Persistent task status</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          Refresh tasks
        </button>
      </div>

      <div v-if="collectionTasks.length" class="task-list">
        <article v-for="task in collectionTasks" :key="task.task_id" class="task-row">
          <div class="task-row__summary">
            <strong>{{ formatTaskType(task.task_type) }}</strong>
            <span>{{ task.status }} - {{ task.succeeded_items }}/{{ task.total_items }} succeeded</span>
            <p>
              waiting {{ task.waiting_items }} - running {{ task.running_items }} -
              failed {{ task.failed_items }} - cancelled {{ task.cancelled_items }}
            </p>
            <p v-if="task.error_message" class="task-row__error">{{ task.error_message }}</p>
          </div>
          <div class="task-row__actions">
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || task.failed_items === 0"
              @click="retryFailedTask(task)"
            >
              Retry failed
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || task.status !== 'running'"
              @click="pauseTask(task)"
            >
              Pause
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || task.status === 'cancelled' || task.status === 'succeeded'"
              @click="cancelTask(task)"
            >
              Cancel
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">No persistent collection tasks recorded yet.</p>
    </section>

    <section class="import-export-grid" aria-label="Import and export Target Official Accounts">
      <div>
        <div class="column-header">
          <strong>Export</strong>
          <button type="button" class="secondary-button" :disabled="isBusy" @click="exportAccounts">
            Export
          </button>
        </div>
        <textarea v-model="exportText" readonly spellcheck="false" />
      </div>

      <div>
        <div class="column-header">
          <strong>Import</strong>
          <button type="button" class="secondary-button" :disabled="isBusy" @click="importAccounts">
            Import
          </button>
        </div>
        <textarea v-model="importText" spellcheck="false" placeholder="Paste exported account JSON" />
      </div>
    </section>
  </section>
</template>
