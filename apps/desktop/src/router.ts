import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import AlbumsView from './views/AlbumsView.vue';
import ArticlesView from './views/ArticlesView.vue';
import SettingsView from './views/SettingsView.vue';
import SingleArticleView from './views/SingleArticleView.vue';
import TargetAccountsView from './views/TargetAccountsView.vue';

export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/target-accounts',
  },
  {
    path: '/target-accounts',
    name: '公众号管理',
    component: TargetAccountsView,
    meta: {
      title: '公众号管理',
      summary: '搜索、添加、导入、导出、同步和删除本地目标公众号。',
    },
  },
  {
    path: '/articles',
    name: '文章下载',
    component: ArticlesView,
    meta: {
      title: '文章下载',
      summary: '浏览已同步文章，下载内容，预览归档，并导出 Markdown 或 HTML。',
    },
  },
  {
    path: '/single-article',
    name: '单篇文章下载',
    component: SingleArticleView,
    meta: {
      title: '单篇文章下载',
      summary: '粘贴公众号文章链接，直接保存、下载和导出单篇内容。',
    },
  },
  {
    path: '/albums',
    name: '合集下载',
    component: AlbumsView,
    meta: {
      title: '合集下载',
      summary: '采集公众号合集文章链接，并批量导出 Markdown 或 HTML。',
    },
  },
  {
    path: '/settings',
    name: '设置',
    component: SettingsView,
    meta: {
      title: '设置',
      summary: '配置归档目录、导出行为、同步参数、登录凭证和高级网络代理。',
    },
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
