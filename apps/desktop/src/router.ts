import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import ArticlesView from './views/ArticlesView.vue';
import PlaceholderView from './views/PlaceholderView.vue';
import TargetAccountsView from './views/TargetAccountsView.vue';

export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/target-accounts',
  },
  {
    path: '/target-accounts',
    name: 'Target Accounts',
    component: TargetAccountsView,
    meta: {
      title: 'Target Accounts',
      summary: 'Search, add, import, export, sync, and remove Target Official Accounts.',
    },
  },
  {
    path: '/articles',
    name: 'Articles',
    component: ArticlesView,
    meta: {
      title: 'Articles',
      summary: 'Browse synchronized article lists, download content, preview archives, and export Markdown or HTML.',
    },
  },
  {
    path: '/single-article',
    name: 'Single Article',
    component: PlaceholderView,
    meta: {
      title: 'Single Article',
      summary: 'Paste a WeChat article URL, collect it locally, and export it without adding the full account first.',
    },
  },
  {
    path: '/albums',
    name: 'Albums',
    component: PlaceholderView,
    meta: {
      title: 'Albums',
      summary: 'Collect album article links and batch export album content as Markdown or HTML.',
    },
  },
  {
    path: '/settings',
    name: 'Settings',
    component: PlaceholderView,
    meta: {
      title: 'Settings',
      summary: 'Configure archive location, export behavior, synchronization, credentials, and advanced network proxy.',
    },
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
