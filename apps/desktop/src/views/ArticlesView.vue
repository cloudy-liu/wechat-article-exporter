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

type ArticleArchivePreview = {
  articleId: string;
  title: string;
  sourceUrl: string;
  sourceHtmlFile: string;
  html: string;
};

type ArticleExportOutcome = {
  articleId: string;
  sourceHtmlFile: string;
  markdownFile?: string | null;
  htmlFile?: string | null;
  taskId: string;
};

const accounts = ref<TargetAccount[]>([]);
const articles = ref<TargetArticle[]>([]);
const selectedFakeid = ref('');
const articleSearchInput = ref('');
const selectedArticleIds = ref<string[]>([]);
const collectionTasks = ref<CollectionTask[]>([]);
const preview = ref<ArticleArchivePreview | null>(null);
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);

const selectedArticles = computed(() =>
  articles.value.filter(article => selectedArticleIds.value.includes(article.article_id)),
);

const filteredArticles = computed(() => {
  const query = articleSearchInput.value.trim().toLowerCase();
  if (!query) {
    return articles.value;
  }

  return articles.value.filter(article =>
    [article.title, article.digest, article.author_name, article.source_url]
      .join(' ')
      .toLowerCase()
      .includes(query),
  );
});

const articleTasks = computed(() =>
  collectionTasks.value.filter(task =>
    ['accountArticleSync', 'articleHtmlDownload', 'export'].includes(task.task_type),
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
      await refreshArticles();
    }
  } catch (error) {
    errorMessage.value = formatError(error);
  }
}

async function refreshArticles() {
  if (!selectedFakeid.value) {
    articles.value = [];
    selectedArticleIds.value = [];
    return;
  }

  articles.value = await invoke<TargetArticle[]>('list_target_articles', { fakeid: selectedFakeid.value });
  selectedArticleIds.value = selectedArticleIds.value.filter(articleId =>
    articles.value.some(article => article.article_id === articleId),
  );
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
  selectedArticleIds.value = [];
  preview.value = null;
  await refreshArticles();
}

function toggleArticleSelection(article: TargetArticle) {
  if (selectedArticleIds.value.includes(article.article_id)) {
    selectedArticleIds.value = selectedArticleIds.value.filter(articleId => articleId !== article.article_id);
  } else {
    selectedArticleIds.value = [...selectedArticleIds.value, article.article_id];
  }
}

function toggleAllFilteredArticles() {
  const visibleIds = filteredArticles.value.map(article => article.article_id);
  if (visibleIds.every(articleId => selectedArticleIds.value.includes(articleId))) {
    selectedArticleIds.value = selectedArticleIds.value.filter(articleId => !visibleIds.includes(articleId));
  } else {
    selectedArticleIds.value = Array.from(new Set([...selectedArticleIds.value, ...visibleIds]));
  }
}

async function downloadArticle(article: TargetArticle) {
  await runArticleAction(`Downloading ${article.title || article.article_id}`, async () => {
    await invoke('download_article_html', {
      request: {
        fakeid: article.target_account_id,
        articleId: article.article_id,
        proxy: null,
      },
    });
    message.value = `Download task finished for ${article.title || article.article_id}`;
  });
}

async function downloadSelectedArticles() {
  await runSelectedArticleAction('Download selected', downloadArticle);
}

async function previewArticle(article: TargetArticle) {
  await runArticleAction(`Loading preview for ${article.title || article.article_id}`, async () => {
    preview.value = await invoke<ArticleArchivePreview>('preview_article_archive', {
      request: {
        articleId: article.article_id,
      },
    });
    message.value = `Preview loaded from ${preview.value.sourceHtmlFile}`;
  });
}

async function exportArticleArchive(article: TargetArticle, format: 'markdown' | 'html') {
  await runArticleAction(`Exporting ${format.toUpperCase()} for ${article.title || article.article_id}`, async () => {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.article_id,
        formats: [format],
      },
    });
    const exportedFile = format === 'markdown' ? outcome.markdownFile : outcome.htmlFile;
    message.value = `Exported ${format.toUpperCase()} to ${exportedFile || outcome.sourceHtmlFile}`;
  });
}

async function exportSelectedArticles(format: 'markdown' | 'html') {
  await runSelectedArticleAction(`Export selected ${format.toUpperCase()}`, article =>
    exportArticleArchive(article, format),
  );
}

async function runSelectedArticleAction(label: string, action: (article: TargetArticle) => Promise<void>) {
  if (selectedArticles.value.length === 0) {
    errorMessage.value = 'Select at least one article.';
    return;
  }

  for (const article of selectedArticles.value) {
    await action(article);
  }
  message.value = `${label} finished for ${selectedArticles.value.length} article${selectedArticles.value.length === 1 ? '' : 's'}`;
}

async function runArticleAction(label: string, action: () => Promise<void>) {
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

function isSelected(article: TargetArticle): boolean {
  return selectedArticleIds.value.includes(article.article_id);
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

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
</script>

<template>
  <section class="articles-workbench" aria-labelledby="articles-workbench-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">Account article workflow</p>
        <h3 id="articles-workbench-title">Articles workbench</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshAccounts">Refresh</button>
    </div>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <div class="articles-layout">
      <section aria-label="Target account selector">
        <div class="column-header">
          <strong>Target Official Accounts</strong>
          <span>{{ accounts.length }}</span>
        </div>
        <div v-if="accounts.length" class="account-list">
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
        <p v-else class="empty-state">No Target Official Accounts saved yet.</p>
      </section>

      <section aria-label="Article table">
        <div class="article-table-toolbar">
          <label>
            <span>Search articles</span>
            <input
              id="article-search-input"
              v-model="articleSearchInput"
              type="search"
              placeholder="Title, digest, author, or URL"
              autocomplete="off"
            />
          </label>
          <div class="article-bulk-actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshArticles">
              Refresh articles
            </button>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="toggleAllFilteredArticles">
              Select visible
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || selectedArticleIds.length === 0"
              @click="downloadSelectedArticles"
            >
              Download selected
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || selectedArticleIds.length === 0"
              @click="exportSelectedArticles('markdown')"
            >
              Export selected Markdown
            </button>
            <button
              type="button"
              class="secondary-button"
              :disabled="isBusy || selectedArticleIds.length === 0"
              @click="exportSelectedArticles('html')"
            >
              Export selected HTML
            </button>
          </div>
        </div>

        <div class="article-workflow-summary">
          <strong>{{ filteredArticles.length }}</strong>
          <span>filtered articles</span>
          <strong>{{ selectedArticleIds.length }}</strong>
          <span>selected</span>
        </div>

        <div v-if="filteredArticles.length" class="workflow-table" role="table" aria-label="Synchronized articles">
          <div class="workflow-table__head" role="row">
            <span role="columnheader">Select</span>
            <span role="columnheader">Article</span>
            <span role="columnheader">Published</span>
            <span role="columnheader">Actions</span>
          </div>
          <article v-for="article in filteredArticles" :key="article.article_id" class="workflow-table__row" role="row">
            <label class="row-check">
              <input type="checkbox" :checked="isSelected(article)" @change="toggleArticleSelection(article)" />
              <span>{{ isSelected(article) ? 'Selected' : 'Select' }}</span>
            </label>
            <div class="article-cell">
              <strong>{{ article.title || article.article_id }}</strong>
              <span>{{ article.author_name || 'Unknown author' }}</span>
              <p>{{ article.digest }}</p>
            </div>
            <span class="article-date">{{ formatUnixTime(article.create_time) }}</span>
            <div class="row-actions">
              <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadArticle(article)">
                Download
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="previewArticle(article)">
                Preview
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportArticleArchive(article, 'markdown')">
                Markdown
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportArticleArchive(article, 'html')">
                HTML
              </button>
            </div>
          </article>
        </div>
        <p v-else class="empty-state">No synchronized articles match the current account and search.</p>
      </section>
    </div>

    <section class="article-preview-panel" aria-label="Archive preview">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">Archive preview</p>
          <h3>{{ preview?.title || 'Downloaded article preview' }}</h3>
        </div>
      </div>
      <iframe v-if="preview" title="Archive preview" :srcdoc="preview.html" />
      <p v-else class="empty-state">Select Preview on a downloaded article to inspect the archived HTML.</p>
    </section>

    <section class="article-sync-panel" aria-label="Task progress">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">Task progress</p>
          <h3>Article workflow tasks</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          Refresh tasks
        </button>
      </div>
      <div v-if="articleTasks.length" class="task-list">
        <article v-for="task in articleTasks" :key="task.task_id" class="task-row">
          <div class="task-row__summary">
            <strong>{{ formatTaskType(task.task_type) }}</strong>
            <span>{{ task.status }} - {{ task.succeeded_items }}/{{ task.total_items }} succeeded</span>
            <p>
              waiting {{ task.waiting_items }} - running {{ task.running_items }} -
              failed {{ task.failed_items }} - cancelled {{ task.cancelled_items }}
            </p>
            <p v-if="task.error_message" class="task-row__error">{{ task.error_message }}</p>
          </div>
        </article>
      </div>
      <p v-else class="empty-state">No article workflow tasks recorded yet.</p>
    </section>
  </section>
</template>
