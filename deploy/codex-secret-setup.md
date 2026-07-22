# Codex 运行凭据配置

这份说明只用于 `huawei2` 首次接入真实 Codex，不要把真实值写回仓库。

## 1. 构建

`deploy/build-production.sh` 会初始化 `vendor/codex` 子模块，构建 ICTHub 后端和固定版本
的 Codex CLI，并将后者放到：

```text
/home/winbeau/xju-icthub/backend/tools/codex
```

## 2. 写入密钥

由管理员在服务器上执行，内容只写入 root 可控文件；下面的占位符不要原样执行：

```bash
sudo install -d -m 0750 /etc/icthub
sudo install -m 0600 /dev/null /etc/icthub/codex-api-key
sudoedit /etc/icthub/codex-api-key
sudo chown winbeau:winbeau /etc/icthub/codex-api-key
```

文件只放 API Key 本身，末尾换行不会影响读取。不要使用 `echo` 把真实 Key 留在 shell
历史中。

## 3. backend/.env

在服务器本地配置，不提交：

```dotenv
ICTHUB_CODEX_ENABLED=true
ICTHUB_CODEX_BIN=tools/codex
ICTHUB_CODEX_HOME=data/codex-home
ICTHUB_CODEX_BASE_URL=https://模型代理/v1
ICTHUB_CODEX_MODEL=管理员确认的模型名
ICTHUB_CODEX_API_KEY_FILE=/etc/icthub/codex-api-key
ICTHUB_CODEX_TIMEOUT_SECS=600
```

Base URL 必须是 HTTP(S) 绝对地址；模型名必须由管理员确认。程序只把 Base URL 的 origin
写入 `agent_runs`，不会保存完整 Key 或请求正文。

## 4. 最小验证

```bash
sudo systemctl restart icthub-backend.service icthub-import-worker.service
sudo systemctl --no-pager --full status icthub-backend.service icthub-import-worker.service
curl --noproxy '*' http://127.0.0.1:8003/api/health
```

先由一个实验室成员上传测试 ZIP，观察 `/api/v1/import-jobs/{id}` 的 `agentRuns` 和
`events`。确认结果后再进行连续任务测试。出现异常时先关闭
`ICTHUB_CODEX_ENABLED` 并重启 Worker，历史本地草稿仍可读取。
