# ICTHub 部署说明

目标主机：`huawei2`，部署目录：`/home/winbeau/xju-icthub`。

## 1. 用户态构建

```bash
git clone https://github.com/winbeau/xju-icthub.git /home/winbeau/xju-icthub
cd /home/winbeau/xju-icthub
./deploy/build-production.sh
cp backend/.env.example backend/.env
```

生产环境变量至少包含：

```dotenv
ICTHUB_BIND_ADDR=127.0.0.1:8003
ICTHUB_DATABASE_URL=sqlite://data/icthub.db?mode=rwc
FEIYUE_AUTH_URL=http://127.0.0.1:8001
ICTHUB_IMPORT_ROOT=uploads/imports
ICTHUB_PROJECT_ROOT=uploads/projects
ICTHUB_IMPORT_MAX_UPLOAD_MB=500
ICTHUB_IMPORT_MAX_UNPACKED_MB=768
ICTHUB_IMPORT_WORKER_POLL_MS=500
ICTHUB_IMPORT_WORKER_LEASE_SECS=120
ICTHUB_FFPROBE_BIN=ffprobe
ICTHUB_FFMPEG_BIN=ffmpeg
ICTHUB_PDFTOPPM_BIN=pdftoppm
ICTHUB_CODEX_ENABLED=false
ICTHUB_CODEX_BIN=tools/codex
ICTHUB_CODEX_HOME=data/codex-home
ICTHUB_CODEX_TIMEOUT_SECS=600
ICTHUB_GITHUB_ENABLED=false
ICTHUB_GITHUB_REPO_PREFIX=ict
ICTHUB_GH_BIN=gh
ICTHUB_GIT_BIN=git
ICTHUB_GITHUB_RUNTIME_ROOT=data/github-runs
ICTHUB_GITHUB_TIMEOUT_SECS=900
RUST_LOG=icthub_server=info,tower_http=info
```

Codex 已固定在 `vendor/codex` 的版本提交。生产构建默认调用 vendored 官方安装器，下载并
校验同版本的 Linux 预编译包，安装到 `backend/tools/codex`。这样保留源码审计和版本锁定，
同时避免 8 GiB 主机在 Thin-LTO 链接阶段 OOM。需要在大内存构建机验证源码构建时，可设置
`ICTHUB_CODEX_BUILD_FROM_SOURCE=true`；该模式会临时规范化上游 `Cargo.lock` 中 workspace
包的 `0.0.0` 版本，拒绝其他锁文件变化，并在构建后恢复原始锁文件。

首测阶段保持 `ICTHUB_CODEX_ENABLED=false`；本地整理草稿仍然可用。
需要启用真实 Agent 时，如果服务器已有 Codex 原生配置目录，可直接在 `backend/.env` 增加：

```dotenv
ICTHUB_CODEX_ENABLED=true
ICTHUB_CODEX_BASE_URL=https://你的模型代理/v1
ICTHUB_CODEX_MODEL=管理员确认的模型名
ICTHUB_CODEX_HOME=/home/winbeau/codex-config
```

当 `ICTHUB_CODEX_HOME` 中已有 Codex 原生 `auth.json` 时，无需复制或导出 API Key；配置目录
应为 `0700`，`auth.json` 和 `config.toml` 应为 `0600`。也可以改用
`ICTHUB_CODEX_API_KEY_FILE=/etc/icthub/codex-api-key`。凭据不要放入仓库、SQLite 或任务目录。
Codex 的子工具环境使用 `core` 环境白名单并启用默认
`KEY/SECRET/TOKEN` 排除，避免把模型凭据泄露给附件中的命令或脚本。`ICTHUB_CODEX_HOME`
只用于隔离 Codex 运行状态。真实调用前还需要人工确认 Base URL、模型名和配额。

生产环境使用独立 Worker，`icthub-backend.service` 已固定
`ICTHUB_IMPORT_WORKER_EMBEDDED=false`。本地开发不启动独立 Worker 时，可以保留默认值
`true`，由 API 进程内嵌运行单个 Worker。

私有源码仓库发布由同一个持久化 Worker 执行，但使用独立的 `github_publications` 队列。
启用前需要安装 GitHub CLI，并将组织 Token 写入服务器上的独立文件：

```dotenv
ICTHUB_GITHUB_ENABLED=true
ICTHUB_GITHUB_OWNER=xjuIcthub
ICTHUB_GITHUB_REPO_PREFIX=ict
ICTHUB_GITHUB_TOKEN_FILE=/etc/icthub/github-token
```

Token 文件必须为 `0600`，内容仅一行；不要执行 `gh auth login`，Worker 只在 `gh` 子进程中
临时注入 `GH_TOKEN`。发布器创建 `private` 仓库，仓库名形如
`ict-0001-project-slug`。推送前会删除 `.git` 和生成目录，拒绝符号链接、常见密钥文件、
疑似密钥内容以及超过 GitHub 100 MiB 限制的单文件。

视频元数据、视频封面和 PDF 首页预览需要：

```bash
sudo apt-get install ffmpeg poppler-utils
```

## 2. systemd 与 Nginx

需要管理员权限：

```bash
sudo install -m 0644 deploy/icthub-backend.service /etc/systemd/system/icthub-backend.service
sudo install -m 0644 deploy/icthub-import-worker.service /etc/systemd/system/icthub-import-worker.service
sudo install -m 0644 deploy/nginx-icthub-http.conf /etc/nginx/sites-available/icthub
sudo ln -sfn /etc/nginx/sites-available/icthub /etc/nginx/sites-enabled/icthub
sudo systemctl daemon-reload
sudo systemctl enable --now icthub-backend.service
sudo systemctl enable --now icthub-import-worker.service
sudo nginx -t
sudo systemctl reload nginx
```

在 DNS 尚未生效时可用 Host 头验证：

```bash
curl --noproxy '*' -H 'Host: icthub.top' http://127.0.0.1/api/health
```

## 3. HTTPS

先将 `icthub.top`（以及需要时的 `www.icthub.top`）A/AAAA 记录指向该服务器。DNS 生效并确认 HTTP 可访问后，使用 webroot 申请证书：

```bash
sudo certbot certonly --webroot \
  -w /home/winbeau/xju-icthub/frontend/dist \
  -d icthub.top -d www.icthub.top
sudo install -m 0644 deploy/nginx-icthub-https.conf /etc/nginx/sites-available/icthub
sudo nginx -t
sudo systemctl reload nginx
```

若不使用 `www` 子域，只为主域申请证书并从 Nginx 的 `server_name` 中删除 `www.icthub.top`。

## 4. 验证

```bash
systemctl --no-pager --full status icthub-backend.service
systemctl --no-pager --full status icthub-import-worker.service
curl --noproxy '*' http://127.0.0.1:8003/api/health
curl --noproxy '*' -H 'Host: icthub.top' http://127.0.0.1/api/v1/projects
```

ICTHub 不保存飞跃密码或本地账号。浏览器的 `/auth/*` 请求由 Nginx 转发到飞跃后端，ICTHub 写接口把 Bearer Token 转发给飞跃 `/auth/me` 校验。
