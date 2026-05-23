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
    errorMessage.value = '请输入有效的公众号文章链接。';
    return;
  }

  await runSingleArticleAction('正在保存单篇文章', async () => {
    const saved = await invoke<SingleArticleArchive>('save_single_article', {
      request: {
        sourceUrl: normalizeUrl(singleArticleUrl.value),
        title: singleArticleTitle.value.trim() || null,
      },
    });
    await refreshSingleArticles();
    selectedArticleId.value = saved.articleId;
    message.value = `已保存 ${saved.title}`;
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
    errorMessage.value = '请先保存单篇文章，再下载 HTML。';
    return;
  }

  await runSingleArticleAction(`正在下载 ${article.title} 的 HTML`, async () => {
    const outcome = await invoke<ArticleHtmlDownloadOutcome>('download_single_article_html', {
      request: {
        articleId: article.articleId,
        proxy: null,
      },
    });
    await refreshSingleArticles();
    message.value = `HTML 已下载到 ${outcome.htmlFile}`;
  });
}

async function previewSingleArticle(article = selectedArticle.value) {
  if (!article) {
    errorMessage.value = '请先保存并下载单篇文章，再预览。';
    return;
  }

  await runSingleArticleAction(`正在加载 ${article.title} 的预览`, async () => {
    preview.value = await invoke<ArticleArchivePreview>('preview_article_archive', {
      request: {
        articleId: article.articleId,
      },
    });
    message.value = `预览已从 ${preview.value.sourceHtmlFile} 加载`;
  });
}

async function exportSingleArticle(format: 'markdown' | 'html', article = selectedArticle.value) {
  if (!article) {
    errorMessage.value = '请先保存并下载单篇文章，再导出。';
    return;
  }

  await runSingleArticleAction(`正在导出 ${article.title} 为 ${format.toUpperCase()}`, async () => {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.articleId,
        formats: [format],
      },
    });
    await refreshSingleArticles();
    const exportedFile = format === 'markdown' ? outcome.markdownFile : outcome.htmlFile;
    message.value = `${format.toUpperCase()} 已导出到 ${exportedFile || outcome.sourceHtmlFile}`;
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
    return '文章 HTML 下载';
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
  <section class="single-article-workbench" aria-labelledby="single-article-title">
    <div class="manager-toolbar">
      <div>
        <p class="section-label">单篇文章工作流</p>
        <h3 id="single-article-title">单篇文章</h3>
      </div>
      <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshSingleArticles">刷新</button>
    </div>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <form class="single-article-form" @submit.prevent="saveSingleArticle">
      <label>
        <span>公众号文章链接</span>
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
        <span>标题</span>
        <input v-model="singleArticleTitle" type="text" placeholder="可选本地标题" autocomplete="off" />
      </label>
      <button type="submit" class="primary-button" :disabled="isBusy">保存文章</button>
    </form>

    <div class="single-article-action-bar" aria-label="已选单篇文章操作">
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="downloadSingleArticleHtml()">
        下载 HTML
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="previewSingleArticle()">
        预览
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="exportSingleArticle('markdown')">
        导出 Markdown
      </button>
      <button type="button" class="secondary-button" :disabled="isBusy || !selectedArticle" @click="exportSingleArticle('html')">
        导出 HTML
      </button>
    </div>

    <div class="single-article-layout">
      <section aria-label="已保存单篇文章">
        <div class="column-header">
          <strong>已保存单篇文章</strong>
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
                {{ article.htmlFile ? 'HTML 已下载' : 'HTML 未下载' }}
                {{ article.markdownFile ? ' - Markdown 已导出' : '' }}
              </small>
            </button>
            <div class="row-actions">
              <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadSingleArticleHtml(article)">
                下载 HTML
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="previewSingleArticle(article)">
                预览
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportSingleArticle('markdown', article)">
                导出 Markdown
              </button>
              <button type="button" class="secondary-button" :disabled="isBusy" @click="exportSingleArticle('html', article)">
                导出 HTML
              </button>
            </div>
          </article>
        </div>
        <p v-else class="empty-state">还没有保存单篇文章。</p>
      </section>

      <section class="article-preview-panel" aria-label="归档预览">
        <div class="manager-toolbar">
          <div>
            <p class="section-label">归档预览</p>
            <h3>{{ preview?.title || selectedArticle?.title || '已下载文章预览' }}</h3>
          </div>
        </div>
        <iframe v-if="preview" title="归档预览" :srcdoc="preview.html" />
        <p v-else class="empty-state">先保存并下载单篇文章，再选择预览。</p>
      </section>
    </div>

    <section class="article-sync-panel" aria-label="任务进度">
      <div class="manager-toolbar">
        <div>
          <p class="section-label">任务进度</p>
          <h3>单篇文章任务</h3>
        </div>
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshCollectionTasks">
          刷新任务
        </button>
      </div>

      <div v-if="singleArticleTasks.length" class="task-list">
        <article v-for="task in singleArticleTasks" :key="task.task_id" class="task-row">
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
      <p v-else class="empty-state">还没有单篇文章任务记录。</p>
    </section>
  </section>
</template>
