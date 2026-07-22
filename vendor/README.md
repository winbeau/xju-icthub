# Vendored runtimes

`vendor/codex` 是 <https://github.com/winbeau/codex> 的 Git submodule，当前固定到
OpenAI Codex `rust-v0.145.0` 对应提交 `25af12f7e61572b0bc18ddb1008be543b91519b0`。

ICTHub 不直接依赖 Codex 仓库中的内部 Rust crate。生产构建从固定提交生成原生 CLI，
Import Worker 仅通过 `codex exec --json --output-schema` 调用，以便升级 Codex 时保持
Axum 业务代码和任务状态机稳定。

更新流程：

1. 先同步 `winbeau/codex` Fork 与上游。
2. 在 `vendor/codex` 检出明确的发布标签。
3. 运行 ICTHub 后端测试和假 Runner 集成测试。
4. 单独提交 submodule 指针变化，不使用浮动 `main`。
