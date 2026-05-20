import assert from 'node:assert/strict';
import {
  ALBUM_DOWNLOAD_FORMAT_OPTIONS,
  DEFAULT_ALBUM_DOWNLOAD_FORMAT,
  isAlbumDownloadFormat,
} from '../utils/download/album-format';

function run() {
  assert.equal(DEFAULT_ALBUM_DOWNLOAD_FORMAT, 'markdown');
  assert.deepEqual(ALBUM_DOWNLOAD_FORMAT_OPTIONS, [
    { label: 'Markdown', value: 'markdown' },
    { label: 'HTML', value: 'html' },
  ]);

  assert.equal(isAlbumDownloadFormat('markdown'), true);
  assert.equal(isAlbumDownloadFormat('html'), true);
  assert.equal(isAlbumDownloadFormat('pdf'), false);
  assert.equal(isAlbumDownloadFormat(undefined), false);
}

run();
