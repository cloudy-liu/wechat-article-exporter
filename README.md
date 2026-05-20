<p align="center">
  <img src="./assets/logo.svg" alt="Logo">
</p>

# wechat-article-exporter fork

这是 `cloudy-liu` 维护的 fork，来源仓库是
[`wechat-article/wechat-article-exporter`](https://github.com/wechat-article/wechat-article-exporter)。

原仓库完整说明请看：
[`wechat-article/wechat-article-exporter#readme`](https://github.com/wechat-article/wechat-article-exporter/blob/master/README.md)。

这个 fork 不重复搬运原仓库的大段介绍，重点记录我们在本地部署、批量归档、Markdown 导出和自动化采集场景中做过的优化。当前工作区的开发策略是：同步参考 `upstream`，但分支、提交、推送和 PR 都只走我们自己的 fork，也就是 `origin`。

## 我们优化了什么

### 本地部署默认走 Nitro 下载代理

相关：[#175](https://github.com/wechat-article/wechat-article-exporter/pull/175)、[#169](https://github.com/wechat-article/wechat-article-exporter/issues/169)

问题背景是，本地部署后浏览器端下载文章内容和资源时，部分场景仍会沿用项目内置的公开代理列表。一旦公开代理节点失效，用户虽然已经把应用跑在本地，文章内容和资源下载仍可能失败。更麻烦的是，旧版本可能已经把这些公开代理地址持久化进浏览器本地存储，例如 `preferences.privateProxyList` 或 legacy `wechat-proxy`，导致升级后仍然误判为“用户配置了私有代理”，从而继续绕过本地代理。

我们在 fork 中把默认策略改为：没有真实私有代理时，浏览器下载默认使用同源 Nitro 代理。实现上会识别历史内置公开代理地址，并把它们当作旧默认值处理；如果代理列表中混有内置公开代理和用户自定义代理，则过滤掉内置项，只保留用户真正配置的私有代理。

这样本地 `yarn dev`、Docker 私有化部署和 Web 部署都不再默认依赖公共代理池，同时不破坏已有的自建代理配置。

### Markdown 导出结果更适合归档

相关：[#156](https://github.com/wechat-article/wechat-article-exporter/pull/156)、[#167](https://github.com/wechat-article/wechat-article-exporter/issues/167)

原始微信公众号 HTML 里混有大量运行时内容：样式块、内联 CSS、底部操作栏、SVG/data URL 图标、头像、脚本和只服务网页渲染的节点。如果直接丢给 Markdown 转换器，生成的 `.md` 很容易在开头带一大段 CSS，正文夹杂无关 UI 文本，表格和正文结构也不稳定。

当前 fork 的处理思路是尽量避开完整运行时页面，优先解析 `window.cgiDataNew`，再用统一 renderer 还原文章正文结构，最后交给 Turndown 转 Markdown。已有的单篇 Markdown 修复还会在转换前清理 `<style>`、`<script>`、`<link>`、底部栏、图标、头像、空节点和样式属性，并补上评论渲染。

目标不是复刻网页视觉效果，而是得到更干净的长期归档内容，方便后续进入脚本、笔记、全文搜索、知识库或其他自动化流程。

### 合集批量下载支持 Markdown / HTML 格式选择

相关：[#166](https://github.com/wechat-article/wechat-article-exporter/issues/166)、[#167](https://github.com/wechat-article/wechat-article-exporter/issues/167)

原仓库的合集页批量下载主要按 HTML 离线归档来处理，这对完整保留页面样式很有用，但对文本归档、批量阅读、AI/知识库导入和自动化采集来说偏重。另一个实际问题是，合集文章导出时需要稳定、可排序、带发布日期的文件名。

这个 fork 在合集详情页增加了导出格式选择：

- 默认导出 `Markdown`，适合文本归档和后续处理。
- 保留 `HTML`，继续支持带资源的离线页面归档。
- Markdown 文件名统一为 `yyyy-MM-dd title.md`。
- HTML 归档目录也复用同样的日期前缀命名。
- 标题里的 `/` 会被替换，避免 zip 内部意外生成多层目录。

具体实现集中在 `utils/download/album-format.ts`：这里定义合集下载格式类型、默认值、下拉选项和文件名构造函数。`pages/dashboard/album.vue` 负责在 UI 上绑定格式选择，`composables/useBatchDownload.ts` 根据选择走两条路径：HTML 继续调用现有资源打包逻辑，Markdown 则解析文章 HTML、渲染正文结构，再用 Turndown 写入 `.md` 文件。

### 合集导出格式增加轻量回归测试

相关：[#166](https://github.com/wechat-article/wechat-article-exporter/issues/166)、[#167](https://github.com/wechat-article/wechat-article-exporter/issues/167)

新增 `test/album_download_format.ts`，锁定目前支持的合集导出格式只有 `markdown` 和 `html`，并确认默认值保持为 `markdown`。这个测试很小，但能防止后续改 UI 选项或默认导出行为时无意改变归档策略。

## 开发与验证

安装和启动方式沿用原仓库：

```bash
corepack enable
corepack prepare yarn@1.22.22 --activate
yarn
yarn dev
```

本 fork 当前常用验证命令：

```bash
node_modules/.bin/jiti.cmd test/album_download_format.ts
yarn build
```

## Remote 策略

- `origin` 是我们的 fork：`https://github.com/cloudy-liu/wechat-article-exporter.git`
- `upstream` 是原仓库：`https://github.com/wechat-article/wechat-article-exporter.git`
- 后续所有功能分支、提交、推送和 PR 都只面向 `origin` / fork。
- 不向 `upstream` 直接推送任何修改。

## License

MIT。许可证继承自原仓库，详见本仓库的 `LICENSE`。
