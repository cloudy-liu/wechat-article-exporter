import { format } from 'date-fns';
import type { DownloadableArticle } from '~/types/types';

export type AlbumDownloadFormat = 'markdown' | 'html';

export const DEFAULT_ALBUM_DOWNLOAD_FORMAT: AlbumDownloadFormat = 'markdown';

export const ALBUM_DOWNLOAD_FORMAT_OPTIONS: { label: string; value: AlbumDownloadFormat }[] = [
  { label: 'Markdown', value: 'markdown' },
  { label: 'HTML', value: 'html' },
];

export function isAlbumDownloadFormat(value: unknown): value is AlbumDownloadFormat {
  return value === 'markdown' || value === 'html';
}

export function buildAlbumArticleArchiveName(article: DownloadableArticle): string {
  return `${format(new Date(+article.date * 1000), 'yyyy-MM-dd')} ${article.title.replace(/\//g, '_')}`;
}

export function buildAlbumMarkdownFilename(article: DownloadableArticle): string {
  return `${buildAlbumArticleArchiveName(article)}.md`;
}
