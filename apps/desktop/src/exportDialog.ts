import { open, save } from '@tauri-apps/plugin-dialog';

export type ArticleExportFormat = 'markdown' | 'html';

export function exportFileExtension(format: ArticleExportFormat): 'md' | 'html' {
  return format === 'markdown' ? 'md' : 'html';
}

export function exportFormatLabel(format: ArticleExportFormat): 'Markdown' | 'HTML' {
  return format === 'markdown' ? 'Markdown' : 'HTML';
}

export function suggestedArticleExportName(
  title: string,
  articleId: string,
  format: ArticleExportFormat,
): string {
  const baseName = safeFileSegment(title || articleId) || safeFileSegment(articleId) || 'article';

  return `${baseName}.${exportFileExtension(format)}`;
}

export async function chooseArticleExportFile(
  title: string,
  articleId: string,
  format: ArticleExportFormat,
): Promise<string | null> {
  const extension = exportFileExtension(format);
  const selected = await save({
    title: `导出 ${exportFormatLabel(format)}`,
    defaultPath: suggestedArticleExportName(title, articleId, format),
    filters: [
      {
        name: exportFormatLabel(format),
        extensions: [extension],
      },
    ],
  });

  return typeof selected === 'string' ? selected : null;
}

export async function chooseArticleExportDirectory(): Promise<string | null> {
  const selected = await open({
    title: '选择导出文件夹',
    directory: true,
    multiple: false,
  });

  return typeof selected === 'string' ? selected : null;
}

function safeFileSegment(value: string): string {
  return value
    .trim()
    .replace(/[<>:"/\\|?*\x00-\x1f]/g, '_')
    .replace(/\s+/g, ' ')
    .replace(/[. ]+$/g, '')
    .slice(0, 120);
}
