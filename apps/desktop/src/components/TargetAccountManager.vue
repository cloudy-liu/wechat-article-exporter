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
      count: 5,
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
    selectedFakeid.value = account.fakeid;
    await refreshArticleList(account.fakeid);
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
    message.value = `已从本地归档删除 ${account.nickname}`;
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
  message.value = `正在同步 ${account.nickname} 的文章`;

  try {
    syncStatus.value = await invoke<ArticleListSyncRecord>('sync_target_account_articles', {
      fakeid: account.fakeid,
      maxItems: maxItems.value,
      pageSize: pageSize.value,
    });
    await refreshArticleList(account.fakeid);
    await refreshCollectionTasks();
    message.value = `已同步 ${syncStatus.value.fetched_count} 篇文章`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}。可以重试同步。`;
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
  message.value = `正在下载 ${article.title || article.article_id} 的 HTML`;

  try {
    const outcome = await invoke<ArticleHtmlDownloadOutcome>('download_article_html', {
      request: {
        fakeid: article.target_account_id,
        articleId: article.article_id,
        proxy: networkProxySetting(),
      },
    });
    await refreshCollectionTasks();
    message.value = `HTML 已下载到 ${outcome.htmlFile}`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}。可在采集任务中查看重试详情。`;
    await refreshCollectionTasks();
  } finally {
    isBusy.value = false;
  }
}

async function exportArticleArchive(article: TargetArticle, format: 'markdown' | 'html') {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = `正在导出 ${article.title || article.article_id} 为 ${format.toUpperCase()}`;

  try {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.article_id,
        formats: [format],
      },
    });
    await refreshCollectionTasks();
    const exportedFile = format === 'markdown' ? outcome.markdownFile : outcome.htmlFile;
    message.value = `${format.toUpperCase()} 已导出到 ${exportedFile || outcome.sourceHtmlFile}`;
  } catch (error) {
    errorMessage.value = `${formatError(error)}。导出前需要先下载这篇文章的 HTML。`;
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
    message.value = '失败项已重新进入可重试状态';
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
    message.value = '采集任务已暂停';
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
    message.value = '采集任务已取消';
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
    message.value = '导出 JSON 已生成';
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
    message.value = '目标公众号已导入';
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
    return '公众号文章同步';
  }
  if (value === 'articleHtmlDownload') {
    return '文章 HTML 下载';
  }
  if (value === 'albumDownload') {
    return '合集下载';
  }
  if (value === 'export') {
    return '导出';
  }

  return value;
}

function formatSyncStatus(value: ArticleListSyncRecord['status']): string {
  if (value === 'running') {
    return '同步中';
  }
  if (value === 'completed') {
    return '已完成';
  }
  if (value === 'failed') {
    return '失败';
  }

  return value;
}

function formatTaskStatus(value: CollectionTask['status'] | CollectionTaskItem['status']): string {
  if (value === 'waiting') {
    return '等待中';
  }
  if (value === 'running') {
    return '运行中';
  }
  if (value === 'paused') {
    return '已暂停';
  }
  if (value === 'cancelled') {
    return '已取消';
  }
  if (value === 'succeeded') {
    return '已成功';
  }
  if (value === 'failed') {
    return '失败';
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
        <p class="section-label">目标公众号</p>
        <h3 id="target-account-manager-title">搜索和管理公众号</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshAccounts">
        刷新
      </button>
    </div>

    <form class="search-row" @submit.prevent="searchAccounts">
      <label>
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

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <div class="account-columns">
      <section aria-label="搜索结果">
        <div class="column-header">
          <strong>搜索结果</strong>
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
              添加
            </button>
          </article>
        </div>
        <p v-else class="empty-state">还没有加载搜索结果。</p>
      </section>

      <section aria-label="本地目标公众号">
        <div class="column-header">
          <strong>本地归档</strong>
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
              删除
            </button>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="syncArticles(account)">
              同步文章
            </button>
          </article>
        </div>
        <p v-else class="empty-state">还没有保存目标公众号。</p>
      </section>
    </div>

    <section class="article-sync-panel" aria-label="文章列表同步">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">同步文章</p>
          <h3>同步已保存公众号的文章</h3>
        </div>
        <button
          type="button"
          class="secondary-button"
          :disabled="isBusy || !selectedFakeid"
          @click="refreshArticleList(selectedFakeid)"
        >
          刷新文章
        </button>
      </div>

      <div class="sync-controls">
        <label>
          <span>历史上限</span>
          <input v-model.number="maxItems" type="number" min="1" max="200" />
        </label>
        <label>
          <span>每页数量</span>
          <input v-model.number="pageSize" type="number" min="1" max="20" />
        </label>
      </div>

      <div class="network-proxy-controls">
        <label>
          <span>网络代理</span>
          <input v-model="proxyUrl" type="url" placeholder="http://127.0.0.1:7890" autocomplete="off" />
        </label>
        <label>
          <span>代理授权</span>
          <input v-model="proxyAuthorization" type="text" placeholder="可选请求头值" autocomplete="off" />
        </label>
      </div>

      <div class="sync-summary">
        <strong>最近同步</strong>
        <span v-if="syncStatus">
          {{ formatSyncStatus(syncStatus.status) }} - 已抓取 {{ syncStatus.fetched_count }}/{{ syncStatus.requested_limit }}
          <span v-if="syncStatus.total_count"> - 平台共 {{ syncStatus.total_count }} 篇</span>
        </span>
        <span v-else>当前公众号还没有同步记录。</span>
        <p v-if="syncStatus?.error_message">
          {{ syncStatus.error_message }}。可以从本地归档公众号行重新同步。
        </p>
      </div>

      <div class="column-header">
        <strong>已同步文章</strong>
        <span>{{ articleList.length }}</span>
      </div>

      <div v-if="articleList.length" class="article-list">
        <article v-for="article in articleList" :key="article.article_id" class="article-row">
          <img v-if="article.cover" :src="article.cover" alt="" />
          <div>
            <strong>{{ article.title }}</strong>
            <span>{{ article.author_name || '未知作者' }} - {{ formatUnixTime(article.create_time) }}</span>
            <p>{{ article.digest }}</p>
          </div>
          <div class="article-row__actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadArticleHtml(article)">
              下载 HTML
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy"
              @click="exportArticleArchive(article, 'markdown')"
            >
              导出 Markdown
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy"
              @click="exportArticleArchive(article, 'html')"
            >
              导出 HTML
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">还没有加载已同步文章。</p>
    </section>

    <section class="article-sync-panel" aria-label="采集任务">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">采集任务</p>
          <h3>本地持久任务状态</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          刷新任务
        </button>
      </div>

      <div v-if="collectionTasks.length" class="task-list">
        <article v-for="task in collectionTasks" :key="task.task_id" class="task-row">
          <div class="task-row__summary">
            <strong>{{ formatTaskType(task.task_type) }}</strong>
            <span>{{ formatTaskStatus(task.status) }} - 已成功 {{ task.succeeded_items }}/{{ task.total_items }}</span>
            <p>
              等待 {{ task.waiting_items }} - 运行 {{ task.running_items }} -
              失败 {{ task.failed_items }} - 取消 {{ task.cancelled_items }}
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
              重试失败项
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || task.status !== 'running'"
              @click="pauseTask(task)"
            >
              暂停
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || task.status === 'cancelled' || task.status === 'succeeded'"
              @click="cancelTask(task)"
            >
              取消
            </button>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">还没有本地采集任务记录。</p>
    </section>

    <section class="import-export-grid" aria-label="导入和导出目标公众号">
      <div>
        <div class="column-header">
          <strong>导出</strong>
          <button type="button" class="secondary-button" :disabled="isBusy" @click="exportAccounts">
            导出
          </button>
        </div>
        <textarea v-model="exportText" readonly spellcheck="false" />
      </div>

      <div>
        <div class="column-header">
          <strong>导入</strong>
          <button type="button" class="secondary-button" :disabled="isBusy" @click="importAccounts">
            导入
          </button>
        </div>
        <textarea v-model="importText" spellcheck="false" placeholder="粘贴已导出的公众号 JSON" />
      </div>
    </section>
  </section>
</template>
