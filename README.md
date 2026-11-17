# SmarFit · 智装工场

面向中小型智能家居服务商的方案工作台。当前版本支持分享客户需求问卷、把收件箱需求转为项目、维护产品库、编制报价、查看历史发布版及打印客户版。

在「需求收件箱」创建并复制问卷链接。客户可直接打开填写，提交后点击「刷新」即可查看。关闭问卷会让旧链接失效，已收到的需求仍可处理。

在项目内打开「Wi-Fi 规划」，输入尺寸、画墙、放置 AP。保存规划后可从产品库选择设备，把 AP 数量加入当前报价草稿。覆盖图使用通用 5 GHz 传播与示例墙损参数，仅供模拟预估，需在安装后现场验证。

线上试验请使用虚构资料。试验工作台的访问凭证保存在当前浏览器，清除站点数据会失去访问入口。正式账号与访问控制上线后再录入真实客户资料。

## 本地运行

需要 Node.js 24、pnpm 10.32.1 和 rustup。Rust 版本由 `rust-toolchain.toml` 固定。

```sh
cargo install worker-build --version 0.9.0 --locked
pnpm install --frozen-lockfile
pnpm dev
```

打开 http://localhost:8787 。修改网页后重新执行 `pnpm build:web`。本地 D1 由 Wrangler 模拟并应用迁移。

```sh
cargo test --locked --workspace
pnpm check:web
node scripts/smoke.mjs http://localhost:8787
```

## Cloudflare 发布

网页和 Rust API 共用一个 Worker。D1 迁移位于 `migrations/`。

1. 在 Cloudflare 启用 Workers 的 `workers.dev` 子域名，分别创建 `smarfit-main` 和 `smarfit-preview` 两个 D1 数据库。
2. 在 GitHub 的 Actions Secrets 中设置 `CLOUDFLARE_API_TOKEN` 和 `CLOUDFLARE_ACCOUNT_ID`。API token 限定目标账户，授予 Workers Scripts 编辑和 D1 编辑权限。
3. 在 Actions Variables 设置 `SMARFIT_D1_MAIN`、`SMARFIT_D1_PREVIEW` 为两个不同的数据库 UUID，最后设置 `SMARFIT_URL` 为 `https://smarfit.<账户子域名>.workers.dev`。
4. 推送到 `main` 后核对 Actions 的发布结果。首次主站发布完成后，仓库内的其他分支通过检查会创建 Cloudflare Preview，可从工作流摘要打开。外部 fork PR 执行检查。

部署后自动检查首页、`/api/health` 和 `/api/version`。检查失败时尝试回滚 Worker。首部署以通过检查的修订重新发布。数据库迁移先在预览验证并保持向后兼容，数据库恢复需在 D1 单独操作。

## 许可证与溯源

SmarFit 源码公开，采用 [PolyForm Noncommercial 1.0.0](LICENSE)。超出公开许可范围的商用须事先联系 [Alex Yip](https://github.com/Yipalex) 获取授权。

源码署名包含 `ximo`，页脚和 `/api/version` 显示构建版本与 Git 提交。维护者可手动运行 GitHub Actions 的 **Build downloadable artifact**，生成带构建来源证明的 Worker 压缩包。下载并解开 Actions 外层 ZIP 后，用 `gh attestation verify smarfit-worker.tar.gz -R Yipalex/SmarFit` 验证。

欢迎在 Issues 反馈使用场景。外部代码贡献在另行商业授权所需的贡献条款就绪后开始合并。参见[路线图](ROADMAP.md)。
