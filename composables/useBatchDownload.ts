import { saveAs } from 'file-saver';
import JSZip from 'jszip';
import TurndownService from 'turndown';
import { parseCgiDataNew } from '#shared/utils/html';
import { renderHTMLFromCgiDataNew } from '#shared/utils/renderer';
import type { DownloadableArticle } from '~/types/types';
import { downloadArticleHTMLs, packHTMLAssets } from '~/utils';
import {
  type AlbumDownloadFormat,
  buildAlbumArticleArchiveName,
  buildAlbumMarkdownFilename,
  DEFAULT_ALBUM_DOWNLOAD_FORMAT,
} from '~/utils/download/album-format';

async function renderMarkdown(article: DownloadableArticle, turndownService: TurndownService): Promise<string> {
  const html = article.html!;
  const cgiData = await parseCgiDataNew(html);
  const renderedHTML = cgiData ? await renderHTMLFromCgiDataNew(cgiData, false) : html;

  return turndownService.turndown(renderedHTML);
}

/**
 * 批量下载合集文章
 */
export function useDownloadAlbum() {
  const loading = ref(false);
  const phase = ref();
  const downloadedCount = ref(0);
  const packedCount = ref(0);

  async function download(
    articles: DownloadableArticle[],
    filename: string,
    format: AlbumDownloadFormat = DEFAULT_ALBUM_DOWNLOAD_FORMAT
  ) {
    loading.value = true;
    downloadedCount.value = 0;
    packedCount.value = 0;

    try {
      phase.value = '下载文章内容';
      const results = await downloadArticleHTMLs(articles, (count: number) => {
        downloadedCount.value = count;
      });

      phase.value = '打包';
      const zip = new JSZip();

      if (format === 'html') {
        for (const article of results) {
          await packHTMLAssets(
            article.fakeid,
            article.html!,
            article.title.replaceAll('.', '_'),
            zip.folder(buildAlbumArticleArchiveName(article))!
          );
          packedCount.value++;
        }
      } else {
        const turndownService = new TurndownService();
        for (const article of results) {
          const markdown = await renderMarkdown(article, turndownService);
          zip.file(buildAlbumMarkdownFilename(article), markdown);
          packedCount.value++;
        }
      }

      const blob = await zip.generateAsync({ type: 'blob' });
      saveAs(blob, `${filename}.zip`);
    } catch (e: any) {
      alert(e.message);
      console.error(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    loading,
    phase,
    downloadedCount,
    packedCount,
    download,
  };
}
