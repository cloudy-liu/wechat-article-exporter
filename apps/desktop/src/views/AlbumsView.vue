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

type TargetArticleAlbumInfo = {
  album_id: number;
  id: string;
  tag_source: number;
  title: string;
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
  album_infos: TargetArticleAlbumInfo[];
};

type AlbumBaseInfo = {
  articleCount: string;
  brandIcon: string;
  cover: string;
  description: string;
  nickname: string;
  title: string;
  username: string;
};

type AlbumPage = {
  albumId: string;
  albumTitle: string;
  baseInfo: AlbumBaseInfo;
  articles: TargetArticle[];
  hasMore: boolean;
};

type AlbumFetchAllOutcome = {
  albumId: string;
  albumTitle: string;
  articles: TargetArticle[];
  taskId: string;
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
};

const accounts = ref<TargetAccount[]>([]);
const albums = ref<TargetArticleAlbumInfo[]>([]);
const albumArticles = ref<TargetArticle[]>([]);
const collectionTasks = ref<CollectionTask[]>([]);
const selectedFakeid = ref('');
const selectedAlbumId = ref('');
const albumPageSize = ref(20);
const albumBaseInfo = ref<AlbumBaseInfo | null>(null);
const hasMoreAlbumPages = ref(false);
const nextBeginMsgid = ref<string | null>(null);
const nextBeginItemidx = ref<string | null>(null);
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);

const selectedAccount = computed(() =>
  accounts.value.find(account => account.fakeid === selectedFakeid.value) || null,
);

const selectedAlbum = computed(() =>
  albums.value.find(album => album.id === selectedAlbumId.value) || null,
);

const albumTasks = computed(() =>
  collectionTasks.value.filter(task =>
    ['albumDownload', 'export'].includes(task.task_type)
      && (!selectedFakeid.value || task.target_account_id === selectedFakeid.value),
  ),
);

onMounted(async () => {
  await refreshAccounts();
  await refreshCollectionTasks();
});

async function refreshAccounts() {
  try {
    accounts.value = await invoke<TargetAccount[]>('list_target_accounts');
    if (!selectedFakeid.value && accounts.value.length > 0) {
      selectedFakeid.value = accounts.value[0].fakeid;
      await refreshAlbums();
    }
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

async function refreshAlbums() {
  if (!selectedFakeid.value) {
    albums.value = [];
    selectedAlbumId.value = '';
    resetAlbumPageState();
    return;
  }

  try {
    albums.value = await invoke<TargetArticleAlbumInfo[]>('list_target_account_albums', {
      fakeid: selectedFakeid.value,
    });
    if (!albums.value.some(album => album.id === selectedAlbumId.value)) {
      selectedAlbumId.value = albums.value[0]?.id || '';
    }
    resetAlbumPageState();
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

async function refreshCollectionTasks() {
  try {
    collectionTasks.value = await invoke<CollectionTask[]>('list_collection_tasks');
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

async function selectAccount(fakeid: string) {
  selectedFakeid.value = fakeid;
  selectedAlbumId.value = '';
  resetAlbumPageState();
  await refreshAlbums();
}

async function selectAlbum(albumId: string) {
  selectedAlbumId.value = albumId;
  resetAlbumPageState();
  if (albumId) {
    await loadAlbumFirstPage();
  }
}

async function loadAlbumFirstPage() {
  await runAlbumAction('正在加载合集文章', async () => {
    const page = await fetchAlbumPage(null, null);
    replaceAlbumPage(page);
    message.value = `已加载 ${page.articles.length} 条文章链接`;
  });
}

async function loadNextAlbumPage() {
  if (!hasMoreAlbumPages.value || !nextBeginMsgid.value || !nextBeginItemidx.value) {
    return;
  }

  await runAlbumAction('正在加载下一页合集文章', async () => {
    const page = await fetchAlbumPage(nextBeginMsgid.value, nextBeginItemidx.value);
    appendAlbumPage(page);
    message.value = `已加载 ${albumArticles.value.length} 条文章链接`;
  });
}

async function fetchAllAlbumArticles() {
  if (!selectedAccount.value || !selectedAlbum.value) {
    errorMessage.value = '请先选择目标公众号和合集。';
    return;
  }

  await runAlbumAction('正在抓取全部文章链接', async () => {
    const outcome = await invoke<AlbumFetchAllOutcome>('fetch_all_album_articles', {
      request: {
        fakeid: selectedAccount.value!.fakeid,
        albumId: selectedAlbum.value!.id,
        albumTitle: selectedAlbum.value!.title,
        pageSize: albumPageSize.value,
        isReverse: false,
      },
    });
    albumArticles.value = outcome.articles;
    updatePaginationFromArticles(outcome.articles, false);
    message.value = `已抓取 ${outcome.articles.length} 条文章链接`;
  });
}

async function downloadAlbumArticles() {
  if (!selectedAccount.value || !selectedAlbum.value) {
    errorMessage.value = '请先选择目标公众号和合集。';
    return;
  }

  await runAlbumAction('正在下载合集 HTML', async () => {
    const outcomes = await invoke<ArticleHtmlDownloadOutcome[]>('download_album_articles', {
      request: {
        fakeid: selectedAccount.value!.fakeid,
        albumId: selectedAlbum.value!.id,
        proxy: null,
      },
    });
    message.value = `已下载 ${outcomes.length} 篇合集文章`;
  });
}

async function exportAlbumArticles(format: 'markdown' | 'html') {
  if (!selectedAccount.value || !selectedAlbum.value) {
    errorMessage.value = '请先选择目标公众号和合集。';
    return;
  }

  await runAlbumAction(`正在导出合集为 ${format.toUpperCase()}`, async () => {
    const outcomes = await invoke<ArticleExportOutcome[]>('export_album_articles', {
      request: {
        fakeid: selectedAccount.value!.fakeid,
        albumId: selectedAlbum.value!.id,
        formats: [format],
      },
    });
    message.value = `已导出 ${outcomes.length} 篇合集文章为 ${format.toUpperCase()}`;
  });
}

async function fetchAlbumPage(beginMsgid: string | null, beginItemidx: string | null): Promise<AlbumPage> {
  if (!selectedAccount.value || !selectedAlbum.value) {
    throw new Error('请先选择目标公众号和合集。');
  }

  return invoke<AlbumPage>('fetch_album_page', {
    request: {
      fakeid: selectedAccount.value.fakeid,
      albumId: selectedAlbum.value.id,
      albumTitle: selectedAlbum.value.title,
      beginMsgid,
      beginItemidx,
      count: albumPageSize.value,
      isReverse: false,
    },
  });
}

async function runAlbumAction(label: string, action: () => Promise<void>) {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = label;

  try {
    await action();
    await refreshCollectionTasks();
  } catch (error) {
    errorMessage.value = formatError(error);
    await refreshCollectionTasks();
  } finally {
    isBusy.value = false;
  }
}

function replaceAlbumPage(page: AlbumPage) {
  albumBaseInfo.value = page.baseInfo;
  albumArticles.value = page.articles;
  updatePaginationFromArticles(page.articles, page.hasMore);
}

function appendAlbumPage(page: AlbumPage) {
  albumBaseInfo.value = page.baseInfo;
  const seen = new Set(albumArticles.value.map(article => article.article_id));
  albumArticles.value = [
    ...albumArticles.value,
    ...page.articles.filter(article => !seen.has(article.article_id)),
  ];
  updatePaginationFromArticles(page.articles, page.hasMore);
}

function updatePaginationFromArticles(articles: TargetArticle[], hasMore: boolean) {
  hasMoreAlbumPages.value = hasMore;
  const last = articles[articles.length - 1];
  nextBeginMsgid.value = last ? String(last.appmsgid) : null;
  nextBeginItemidx.value = last ? String(last.itemidx) : null;
}

function resetAlbumPageState() {
  albumArticles.value = [];
  albumBaseInfo.value = null;
  hasMoreAlbumPages.value = false;
  nextBeginMsgid.value = null;
  nextBeginItemidx.value = null;
}

function formatUnixTime(value: number): string {
  if (!value) {
    return '--';
  }

  return new Date(value * 1000).toLocaleString();
}

function formatTaskType(value: CollectionTask['task_type']): string {
  if (value === 'albumDownload') {
    return '合集下载';
  }
  if (value === 'export') {
    return '导出';
  }

  return value;
}

function formatTaskStatus(value: CollectionTask['status']): string {
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

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
</script>

<template>
  <section class="albums-workbench" aria-labelledby="albums-workbench-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">合集工作流</p>
        <h3 id="albums-workbench-title">合集下载</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshAccounts">刷新</button>
    </div>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <div class="albums-layout">
      <section aria-label="合集公众号选择">
        <div class="column-header">
          <strong>目标公众号</strong>
          <span>{{ accounts.length }}</span>
        </div>
        <div v-if="accounts.length" id="album-account-selector" class="account-list">
          <button
            v-for="account in accounts"
            :key="account.fakeid"
            type="button"
            class="account-choice"
            :class="{ active: selectedFakeid === account.fakeid }"
            :disabled="isBusy"
            @click="selectAccount(account.fakeid)"
          >
            <img v-if="account.round_head_img" :src="account.round_head_img" alt="" />
            <span>
              <strong>{{ account.nickname }}</strong>
              <small>{{ account.alias || account.fakeid }}</small>
            </span>
          </button>
        </div>
        <p v-else class="empty-state">还没有保存目标公众号。</p>
      </section>

      <section aria-label="合集选择">
        <div class="column-header">
          <strong>合集</strong>
          <span>{{ albums.length }}</span>
        </div>
        <div v-if="albums.length" class="album-choice-list">
          <button
            v-for="album in albums"
            :key="album.id"
            type="button"
            class="album-choice"
            :class="{ active: selectedAlbumId === album.id }"
            :disabled="isBusy"
            @click="selectAlbum(album.id)"
          >
            <strong># {{ album.title || album.id }}</strong>
            <span>{{ album.id }}</span>
          </button>
        </div>
        <p v-else class="empty-state">请先同步公众号文章，再发现可用合集。</p>
      </section>

      <section aria-label="合集文章工作流">
        <div class="album-action-bar">
          <label>
            <span>每页数量</span>
            <input v-model.number="albumPageSize" type="number" min="1" max="50" />
          </label>
          <button type="button" class="secondary-button" :disabled="isBusy || !selectedAlbum" @click="loadAlbumFirstPage">
            加载第一页
          </button>
          <button
            type="button"
            class="secondary-button"
            :disabled="isBusy || !hasMoreAlbumPages"
            @click="loadNextAlbumPage"
          >
            加载下一页
          </button>
          <button type="button" class="secondary-button" :disabled="isBusy || !selectedAlbum" @click="fetchAllAlbumArticles">
            抓取全部文章链接
          </button>
          <button
            type="button"
            class="secondary-button"
            :disabled="isBusy || !selectedAlbum || albumArticles.length === 0"
            @click="downloadAlbumArticles"
          >
            下载合集 HTML
          </button>
          <button
            type="button"
            class="secondary-button"
            :disabled="isBusy || !selectedAlbum || albumArticles.length === 0"
            @click="exportAlbumArticles('markdown')"
          >
            导出 Markdown
          </button>
          <button
            type="button"
            class="secondary-button"
            :disabled="isBusy || !selectedAlbum || albumArticles.length === 0"
            @click="exportAlbumArticles('html')"
          >
            导出 HTML
          </button>
        </div>

        <div v-if="albumBaseInfo" class="album-summary">
          <img v-if="albumBaseInfo.cover || albumBaseInfo.brandIcon" :src="albumBaseInfo.cover || albumBaseInfo.brandIcon" alt="" />
          <div>
            <strong>{{ albumBaseInfo.title || selectedAlbum?.title }}</strong>
            <span>{{ albumBaseInfo.nickname || selectedAccount?.nickname }}</span>
            <p>
              已加载 {{ albumArticles.length }} 篇
              <span v-if="albumBaseInfo.articleCount">/ 共 {{ albumBaseInfo.articleCount }} 篇</span>
              <span v-if="albumBaseInfo.description"> - {{ albumBaseInfo.description }}</span>
            </p>
          </div>
        </div>

        <div v-if="albumArticles.length" class="workflow-table album-table" role="table" aria-label="合集文章">
          <div class="workflow-table__head" role="row">
            <span role="columnheader">文章</span>
            <span role="columnheader">发布时间</span>
            <span role="columnheader">来源</span>
          </div>
          <article v-for="article in albumArticles" :key="article.article_id" class="workflow-table__row" role="row">
            <div class="article-cell">
              <strong>{{ article.title || article.article_id }}</strong>
              <span>{{ article.article_id }} - msg {{ article.appmsgid }}/{{ article.itemidx }}</span>
            </div>
            <span class="article-date">{{ formatUnixTime(article.create_time) }}</span>
            <a :href="article.source_url" target="_blank" rel="noreferrer">{{ article.source_url }}</a>
          </article>
        </div>
        <p v-else class="empty-state">请选择合集，然后加载或抓取文章链接。</p>
      </section>
    </div>

    <section class="article-sync-panel" aria-label="任务进度">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">任务进度</p>
          <h3>合集工作流任务</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          刷新任务
        </button>
      </div>

      <div v-if="albumTasks.length" class="task-list">
        <article v-for="task in albumTasks" :key="task.task_id" class="task-row">
          <div class="task-row__summary">
            <strong>{{ formatTaskType(task.task_type) }}</strong>
            <span>{{ formatTaskStatus(task.status) }} - 已成功 {{ task.succeeded_items }}/{{ task.total_items }}</span>
            <p>
              等待 {{ task.waiting_items }} - 运行 {{ task.running_items }} -
              失败 {{ task.failed_items }} - 取消 {{ task.cancelled_items }}
            </p>
            <p v-if="task.error_message" class="task-row__error">{{ task.error_message }}</p>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">还没有合集工作流任务记录。</p>
    </section>
  </section>
</template>
