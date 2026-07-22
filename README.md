# 新疆大学 ICT&软开实验室主页

`xju-icthub` 是新疆大学 ICT&软开实验室的统一站点仓库，目标域名为 `icthub.top`。

当前首期建设内容是其中的“项目库”模块，用于统一整理实验室历届与在研项目，记录项目类别、内容简介、曾获奖、负责人、状态与相关资源，并提供快速导入、检索和 Agent 问答能力。

## 站点与模块定位

- `/`：实验室主页，首期保留站点壳与内容占位；
- `/projects`：项目库列表，当前主要开发页面；
- `/projects/:slug`：项目详情；
- `/admin/projects`：项目管理入口；
- `/admin/projects/new`：新建项目；
- `/admin/projects/:slug/edit`：编辑项目；
- 未来可增加成员、研究方向、成果、加入我们等全局导航。

全局顶部导航将作为独立站点壳预留。首期不在项目页面中写死完整实验室导航，避免后续扩展时重构页面层级。

## 当前阶段

项目已经进入实现阶段。React/Rust 工程骨架、飞跃登录复用、简约项目列表与详情、项目创建/编辑/归档、资源维护、表格快速导入、SQLite 迁移和身份适配器已经落地。一键导入现已具备持久化 Worker、Office/PDF/媒体抽取、清洗后的分析包，以及固定版本 Codex CLI 的结构化 Agent 运行器；真实模型调用仍需在服务器注入 Base URL、模型名和密钥文件后开启。

- [项目开题文档](docs/项目开题文档.md)
- [开发计划](docs/开发计划.md)
- [里程碑与验收](docs/里程碑.md)
- [与 xju-feiyue 的认证及部署集成](docs/认证与部署集成.md)
- [xju-feiyue 前端复用记录](docs/前端复用记录.md)
- [一键导入与 Codex Agent 计划](docs/一键导入原型计划.md)
- [huawei2 部署说明](deploy/README.md)
- [项目库交互原型](docs/prototypes/02-editorial-list.html)
- [前端设计参考：XjuSelab/xju-feiyue](https://github.com/XjuSelab/xju-feiyue)

## 初步技术方向

- 前端：React、TypeScript、pnpm、Vite、Tailwind CSS、shadcn/ui、TanStack Query、Zod
- 后端：Rust、Axum、Tokio、SQLx、SQLite
- 认证：复用 `xju-feiyue` 账号、JWT 与登录审计；飞跃超级管理员标记实验室成员
- 文件：本地对象目录起步，保留 S3/MinIO 兼容接口
- Agent：固定版本 Codex CLI + 外部模型 API，使用清洗后的只读分析目录和严格 JSON Schema
- Python：仅在必要时使用，依赖与执行统一由 uv 管理
- 部署：`huawei2` 上沿用 systemd + Nginx，域名 `icthub.top`

## 计划目录

```text
xju-icthub/
├── frontend/        实验室站点壳与各业务模块
├── backend/         Rust + Axum API
├── docs/            开题、架构、需求与部署文档
├── deploy/          systemd、Nginx 与部署脚本
└── README.md
```

## 本地开发

前端：

```bash
cd frontend
pnpm install --frozen-lockfile
pnpm dev
```

默认开发模式使用内置 mock。需要连接本地服务时设置 `VITE_USE_MOCK=false`，Vite 会把 `/api` 转发到 `127.0.0.1:8003`、把 `/auth` 转发到飞跃 `127.0.0.1:8001`。

后端：

```bash
cd backend
cargo run -p icthub-server
```

后端默认创建 `backend/data/icthub.db`，监听 `127.0.0.1:8003`。配置示例见 `backend/.env.example`。

项目列表、详情、上传与导入 API 均要求飞跃登录且具有实验室成员身份；创建、编辑、归档和标签管理继续遵循成员、项目关系与管理员权限。批量导入接受前端校验后的结构化项目数组，使用单个 SQLite 事务按 `slug` 新增或更新。

完整检查：

```bash
make check
```

## License

ICTHub 项目自身许可证待确认。已复用组件的许可说明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
