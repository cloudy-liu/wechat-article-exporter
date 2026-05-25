import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function assertHasAll(source: string, markers: string[], label: string) {
  for (const marker of markers) {
    assert.match(source, new RegExp(marker), `${label} should include Chinese UI copy: ${marker}`);
  }
}

const indexHtml = read('apps/desktop/index.html');
assert.match(indexHtml, /<html lang="zh-CN">/);
assert.match(indexHtml, /<title>公众号文章导出工具<\/title>/);

const tauriConfig = JSON.parse(read('apps/desktop/src-tauri/tauri.conf.json'));
assert.equal(tauriConfig.productName, '公众号文章导出工具');
assert.equal(tauriConfig.app.windows[0].title, '公众号文章导出工具');

const cargoToml = read('apps/desktop/src-tauri/Cargo.toml');
assert.match(cargoToml, /description = "面向公众号作者的本地桌面采集和导出工具"/);

const router = read('apps/desktop/src/router.ts');
assertHasAll(
  router,
  [
    '公众号管理',
    '文章下载',
    '单篇文章下载',
    '合集下载',
    '设置',
    '浏览已同步文章',
    '采集公众号合集文章链接',
    '配置归档目录',
  ],
  'router',
);

const app = read('apps/desktop/src/App.vue');
assertHasAll(app, ['本地桌面端', '公众号文章导出工具', '桌面端核心功能'], 'app shell');

const officialAccountLogin = read('apps/desktop/src/components/OfficialAccountLoginPanel.vue');
assertHasAll(
  officialAccountLogin,
  ['公众号平台登录', '不支持个人微信号登录', '加载二维码', '开始登录', '退出登录'],
  'OfficialAccountLoginPanel',
);

const targetAccountManager = read('apps/desktop/src/components/TargetAccountManager.vue');
assertHasAll(
  targetAccountManager,
  [
    '搜索和管理公众号',
    '公众号名称或关键词',
    '搜索结果',
    '本地归档',
    '批量导入',
    '批量导出',
    '删除',
    '同步',
  ],
  'TargetAccountManager',
);

const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
assertHasAll(targetAccountsView, ['desktop-data-page', 'TargetAccountManager'], 'TargetAccountsView');

const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
assertHasAll(
  articlesView,
  ['文章下载操作区', '搜索文章', '导出选中 Markdown', '文章列表', '筛选结果', '归档预览'],
  'ArticlesView',
);

const singleArticleView = read('apps/desktop/src/views/SingleArticleView.vue');
assertHasAll(
  singleArticleView,
  ['请输入公众号文章链接', '公众号文章链接', '保存文章', '已保存单篇文章', '单篇文章任务'],
  'SingleArticleView',
);

const albumsView = read('apps/desktop/src/views/AlbumsView.vue');
assertHasAll(
  albumsView,
  ['合集下载操作区', '合集文章工作流', '抓取全部文章链接', '下载合集 HTML', '合集工作流任务'],
  'AlbumsView',
);

const settingsView = read('apps/desktop/src/views/SettingsView.vue');
assertHasAll(
  settingsView,
  [
    '导出偏好',
    '同步与下载',
    '网络代理',
    '清理凭证',
    '保存设置',
  ],
  'SettingsView',
);
