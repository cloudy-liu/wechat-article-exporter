<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

type SingleArticleArchive = {
  articleId: string;
  targetAccountId: string;
  title: string;
  sourceUrl: string;
  htmlFile?: string | null;
  markdownFile?: string | null;
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

type ArticleHtmlDownloadOutcome = {
  articleId: string;
  targetAccountId: string;
  sourceUrl: string;
  htmlFile: string;
  assetFiles: string[];
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

const singleArticleUrl = ref('');
const singleArticleTitle = ref('');
const singleArticles = ref<SingleArticleArchive[]>([]);
const selectedArticleId = ref('');
const collectionTasks = ref<CollectionTask[]>([]);
const preview = ref<ArticleArchivePreview | null>(null);
const message = ref('');
const errorMessage = ref('');
const isBusy = ref(false);

const selectedArticle = computed(() =>
  singleArticles.value.find(article => article.articleId === selectedArticleId.value) || singleArticles.value[0] || null,
);

const singleArticleTasks = computed(() =>
  collectionTasks.value.filter(task =>
    ['articleHtmlDownload', 'export'].includes(task.task_type) && task.target_account_id === 'single-article',
  ),
);

onMounted(async () => {
  await refreshSingleArticles();
  await refreshCollectionTasks();
});

function validateSingleArticleUrl(value = singleArticleUrl.value): boolean {
  try {
    const candidate = normalizeUrl(value);
    const url = new URL(candidate);
    const hasSlugPath = url.hostname === 'mp.weixin.qq.com' && url.pathname.startsWith('/s/') && url.pathname.length > 3;
    const hasSnQuery = url.hostname === 'mp.weixin.qq.com' && Boolean(url.searchParams.get('sn'));
    return hasSlugPath || hasSnQuery;
  } catch (_error) {
    return false;
  }
}

async function saveSingleArticle() {
  if (!validateSingleArticleUrl()) {
    errorMessage.value = 'Enter a valid WeChat Official Account article URL.';
    return;
  }

  await runSingleArticleAction('Saving single article', async () => {
    const saved = await invoke<SingleArticleArchive>('save_single_article', {
      request: {
        sourceUrl: normalizeUrl(singleArticleUrl.value),
        title: singleArticleTitle.value.trim() || null,
      },
    });
    await refreshSingleArticles();
    selectedArticleId.value = saved.articleId;
    message.value = `Saved ${saved.title}`;
  });
}

async function refreshSingleArticles() {
  try {
    singleArticles.value = await invoke<SingleArticleArchive[]>('list_single_articles');
    if (!selectedArticleId.value && singleArticles.value.length > 0) {
      selectedArticleId.value = singleArticles.value[0].articleId;
    }
    if (!singleArticles.value.some(article => article.articleId === selectedArticleId.value)) {
      selectedArticleId.value = singleArticles.value[0]?.articleId || '';
    }
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

async function downloadSingleArticleHtml(article = selectedArticle.value) {
  if (!article) {
    errorMessage.value = 'Save a single article before downloading HTML.';
    return;
  }

  await runSingleArticleAction(`Downloading HTML for ${article.title}`, async () => {
    const outcome = await invoke<ArticleHtmlDownloadOutcome>('download_single_article_html', {
      request: {
        articleId: article.articleId,
        proxy: null,
      },
    });
    await refreshSingleArticles();
    message.value = `Downloaded HTML to ${outcome.htmlFile}`;
  });
}

async function previewSingleArticle(article = selectedArticle.value) {
  if (!article) {
    errorMessage.value = 'Save and download a single article before previewing.';
    return;
  }

  await runSingleArticleAction(`Loading preview for ${article.title}`, async () => {
    preview.value = await invoke<ArticleArchivePreview>('preview_article_archive', {
      request: {
        articleId: article.articleId,
      },
    });
    message.value = `Preview loaded from ${preview.value.sourceHtmlFile}`;
  });
}

async function exportSingleArticle(format: 'markdown' | 'html', article = selectedArticle.value) {
  if (!article) {
    errorMessage.value = 'Save and download a single article before exporting.';
    return;
  }

  await runSingleArticleAction(`Exporting ${format.toUpperCase()} for ${article.title}`, async () => {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.articleId,
        formats: [format],
      },
    });
    await refreshSingleArticles();
    const exportedFile = format === 'markdown' ? outcome.markdownFile : outcome.htmlFile;
    message.value = `Exported ${format.toUpperCase()} to ${exportedFile || outcome.sourceHtmlFile}`;
  });
}

async function runSingleArticleAction(label: string, action: () => Promise<void>) {
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

function selectSingleArticle(article: SingleArticleArchive) {
  selectedArticleId.value = article.articleId;
  preview.value = null;
}

function normalizeUrl(value: string): string {
  const trimmed = value.trim();
  if (trimmed.startsWith('//')) {
    return `https:${trimmed}`;
  }
  if (trimmed.includes('://')) {
    return trimmed;
  }

  return `https://${trimmed}`;
}

function formatTaskType(value: CollectionTask['task_type']): string {
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
  <section class="single-article-workbench" aria-labelledby="single-article-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">Single article workflow</p>
        <h3 id="single-article-title">Single Article</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshSingleArticles">Refresh</button>
    </div>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <form class="single-article-form" @submit.prevent="saveSingleArticle">
      <label>
        <span>WeChat article URL</span>
        <input
          id="single-article-url-input"
          v-model="singleArticleUrl"
          type="url"
          placeholder="https://mp.weixin.qq.com/s/..."
          autocomplete="off"
          :aria-invalid="singleArticleUrl.length > 0 && !validateSingleArticleUrl()"
        />
      </label>
      <label>
        <span>Title</span>
        <input v-model="singleArticleTitle" type="text" placeholder="Optional local title" autocomplete="off" />
      </label>
      <button type="submit" class="primary-button" :disabled="isBusy">Save article</button>
    </form>

    <div class="single-article-action-bar" aria-label="Selected single article actions">
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="downloadSingleArticleHtml()">
        Download HTML
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="previewSingleArticle()">
        Preview
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="exportSingleArticle('markdown')">
        Export Markdown
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="exportSingleArticle('html')">
        Export HTML
      </button>
    </div>

    <div class="single-article-layout">
      <section aria-label="Saved single articles">
        <div class="column-header">
          <strong>Saved single articles</strong>
          <span>{{ singleArticles.length }}</span>
        </div>

        <div v-if="singleArticles.length" class="single-article-list">
          <article
            v-for="article in singleArticles"
            :key="article.articleId"
            class="single-article-row"
            :class="{ active: selectedArticleId === article.articleId }"
          >
            <button type="button" :disabled="isBusy" @click="selectSingleArticle(article)">
              <strong>{{ article.title }}</strong>
              <span>{{ article.sourceUrl }}</span>
              <small>
                {{ article.htmlFile ? 'HTML downloaded' : 'HTML not downloaded' }}
                {{ article.markdownFile ? ' - Markdown exported' : '' }}
              </small>
            </button>
            <div class="row-actions">
              <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadSingleArticleHtml(article)">
                Download HTML
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="previewSingleArticle(article)">
                Preview
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportSingleArticle('markdown', article)">
                Export Markdown
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportSingleArticle('html', article)">
                Export HTML
              </button>
            </div>
          </article>
        </div>
        <p v-else class="empty-state">No single articles saved yet.</p>
      </section>

      <section class="article-preview-panel" aria-label="Archive preview">
        <div class="manager-toolbar">
          <div>
            <p class="section-label">Archive preview</p>
            <h3>{{ preview?.title || selectedArticle?.title || 'Downloaded article preview' }}</h3>
          </div>
        </div>
        <iframe v-if="preview" title="Archive preview" :srcdoc="preview.html" />
        <p v-else class="empty-state">Save and download a single article, then select Preview.</p>
      </section>
    </div>

    <section class="article-sync-panel" aria-label="Task progress">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">Task progress</p>
          <h3>Single article tasks</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          Refresh tasks
        </button>
      </div>

      <div v-if="singleArticleTasks.length" class="task-list">
        <article v-for="task in singleArticleTasks" :key="task.task_id" class="task-row">
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
      <p v-else class="empty-state">No single article tasks recorded yet.</p>
    </section>
  </section>
</template>
