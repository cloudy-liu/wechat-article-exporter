<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { chooseArticleExportDirectory, chooseArticleExportFile } from '../exportDialog';

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
  savedMarkdownFile?: string | null;
  savedHtmlFile?: string | null;
  taskId: string;
};

const accounts = ref<TargetAccount[]>([]);
const articles = ref<TargetArticle[]>([]);
const selectedFakeid = ref('');
const articleSearchInput = ref('');
const selectedArticleIds = ref<string[]>([]);
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

onMounted(async () => {
  await refreshAccounts();
});

async function handleAccountSelect(event: Event) {
  await selectAccount((event.target as HTMLSelectElement).value);
}

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
  await runArticleAction(`正在下载 ${article.title || article.article_id}`, async () => {
    await invoke('download_article_html', {
      request: {
        fakeid: article.target_account_id,
        articleId: article.article_id,
        proxy: null,
      },
    });
    message.value = `${article.title || article.article_id} 的下载任务已完成`;
  });
}

async function downloadSelectedArticles() {
  await runSelectedArticleAction('下载选中文章', downloadArticle);
}

async function previewArticle(article: TargetArticle) {
  await runArticleAction(`正在加载 ${article.title || article.article_id} 的预览`, async () => {
    preview.value = await invoke<ArticleArchivePreview>('preview_article_archive', {
      request: {
        articleId: article.article_id,
      },
    });
    message.value = `预览已从 ${preview.value.sourceHtmlFile} 加载`;
  });
}

function closePreview() {
  preview.value = null;
}

async function exportArticleArchive(
  article: TargetArticle,
  format: 'markdown' | 'html',
  outputFile?: string | null,
  outputDir?: string | null,
) {
  const selectedOutputFile = outputFile === undefined
    ? await chooseArticleExportFile(article.title || article.article_id, article.article_id, format)
    : outputFile;
  if (!selectedOutputFile && !outputDir) {
    message.value = '已取消导出';
    return;
  }

  await runArticleAction(`正在导出 ${article.title || article.article_id} 为 ${format.toUpperCase()}`, async () => {
    const outcome = await invoke<ArticleExportOutcome>('export_article_archive', {
      request: {
        articleId: article.article_id,
        formats: [format],
        outputFile: selectedOutputFile || null,
        outputDir: outputDir || null,
      },
    });
    const exportedFile = format === 'markdown'
      ? outcome.savedMarkdownFile || outcome.markdownFile
      : outcome.savedHtmlFile || outcome.htmlFile;
    message.value = `${format.toUpperCase()} 已导出到 ${exportedFile || outcome.sourceHtmlFile}`;
  });
}

async function exportSelectedArticles(format: 'markdown' | 'html') {
  const outputDir = await chooseArticleExportDirectory();
  if (!outputDir) {
    message.value = '已取消导出';
    return;
  }

  await runSelectedArticleAction(`导出选中 ${format.toUpperCase()}`, article =>
    exportArticleArchive(article, format, null, outputDir),
  );
  message.value = `导出选中 ${format.toUpperCase()}已完成，共 ${selectedArticles.value.length} 篇文章，保存到 ${outputDir}`;
}

async function runSelectedArticleAction(label: string, action: (article: TargetArticle) => Promise<void>) {
  if (selectedArticles.value.length === 0) {
    errorMessage.value = '请至少选择一篇文章。';
    return;
  }

  for (const article of selectedArticles.value) {
    await action(article);
  }
  message.value = `${label}已完成，共 ${selectedArticles.value.length} 篇文章`;
}

async function runArticleAction(label: string, action: () => Promise<void>) {
  isBusy.value = true;
  errorMessage.value = '';
  message.value = label;

  try {
    await action();
  } catch (error) {
    errorMessage.value = formatError(error);
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

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
</script>

<template>
  <section class="articles-workbench desktop-data-page" aria-labelledby="articles-workbench-title">
    <header class="desktop-page-toolbar" aria-label="文章下载操作区">
      <div class="desktop-control-group">
        <label class="desktop-field">
          <span>公众号</span>
          <select id="article-account-selector" v-model="selectedFakeid" :disabled="isBusy" @change="handleAccountSelect">
            <option value="">请选择公众号</option>
            <option v-for="account in accounts" :key="account.fakeid" :value="account.fakeid">
              {{ account.nickname || account.fakeid }}
            </option>
          </select>
        </label>
        <label class="desktop-field">
          <span>搜索文章</span>
          <input
            id="article-search-input"
            v-model="articleSearchInput"
            type="search"
            placeholder="标题、摘要、作者或链接"
            autocomplete="off"
          />
        </label>
      </div>
      <div class="desktop-bulk-toolbar">
        <button type="button" class="secondary-button" :disabled="isBusy" @click="refreshAccounts">刷新公众号</button>
        <button type="button" class="secondary-button" :disabled="isBusy || !selectedFakeid" @click="refreshArticles">
          刷新文章
        </button>
        <button type="button" class="secondary-button" :disabled="isBusy || filteredArticles.length === 0" @click="toggleAllFilteredArticles">
          选择当前列表
        </button>
        <button
          type="button"
          class="secondary-button"
          aria-label="下载选中文章"
          :disabled="isBusy || selectedArticleIds.length === 0"
          @click="downloadSelectedArticles"
        >
          抓取
        </button>
        <button
          type="button"
          class="secondary-button"
          aria-label="导出选中 Markdown"
          :disabled="isBusy || selectedArticleIds.length === 0"
          @click="exportSelectedArticles('markdown')"
        >
          导出 Markdown
        </button>
        <button
          type="button"
          class="secondary-button"
          aria-label="导出选中 HTML"
          :disabled="isBusy || selectedArticleIds.length === 0"
          @click="exportSelectedArticles('html')"
        >
          导出 HTML
        </button>
      </div>
    </header>

    <p v-if="message" class="manager-message">{{ message }}</p>
    <p v-if="errorMessage" class="manager-error">{{ errorMessage }}</p>

    <section class="desktop-table-shell legacy-article-table-shell" aria-label="文章列表">
      <div class="article-workflow-summary">
        <strong>{{ filteredArticles.length }}</strong>
        <span>筛选结果</span>
        <strong>{{ selectedArticleIds.length }}</strong>
        <span>已选择</span>
      </div>

      <div v-if="filteredArticles.length" class="workflow-table desktop-grid legacy-article-grid" role="table" aria-label="已同步文章">
        <div class="workflow-table__head" role="row">
          <span role="columnheader">选择</span>
          <span role="columnheader">文章</span>
          <span role="columnheader">发布时间</span>
          <span role="columnheader">操作</span>
        </div>
        <article v-for="article in filteredArticles" :key="article.article_id" class="workflow-table__row" role="row">
          <label class="row-check">
            <input type="checkbox" :checked="isSelected(article)" @change="toggleArticleSelection(article)" />
            <span>{{ isSelected(article) ? '已选' : '选择' }}</span>
          </label>
          <div class="article-cell">
            <strong>{{ article.title || article.article_id }}</strong>
            <span>{{ article.author_name || '未知作者' }}</span>
            <p>{{ article.digest }}</p>
          </div>
          <span class="article-date">{{ formatUnixTime(article.create_time) }}</span>
          <div class="row-actions">
            <button type="button" class="secondary-button" :disabled="isBusy" @click="downloadArticle(article)">
              下载
            </button>
            <button type="button" class="secondary-button" :disabled="isBusy" @click="previewArticle(article)">
              预览
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
      <p v-else class="empty-state">当前公众号和搜索条件下没有已同步文章。</p>
    </section>

    <div v-if="preview" class="article-preview-dialog-backdrop" role="presentation" @click.self="closePreview">
      <div class="article-preview-dialog" role="dialog" aria-modal="true" aria-label="归档预览">
        <header class="article-preview-dialog__header">
          <div>
            <p class="section-label">归档预览</p>
            <h3>{{ preview.title || '已下载文章预览' }}</h3>
          </div>
          <button type="button" class="secondary-button" @click="closePreview">关闭</button>
        </header>
        <iframe title="归档预览" :srcdoc="preview.html" />
      </div>
    </div>
  </section>
</template>
