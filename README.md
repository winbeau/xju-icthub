# XJU ICTHub

新疆大学实验室项目资源库与项目知识管理平台。

ICTHub 用于统一整理实验室历届与在研项目，记录项目类别、内容简介、曾获奖、负责人、状态与相关资源，并提供快速导入、检索和 Agent 问答能力。

## 当前阶段

项目处于开题与架构设计阶段，默认前端方向采用简约的“编辑部列表”：首页突出项目名称、内容简介、主类别与曾获奖，其他管理字段进入项目详情。

- [项目开题文档](docs/项目开题文档.md)
- [前端交互原型](docs/prototypes/02-editorial-list.html)
- [参考项目：XjuSelab/xju-feiyue](https://github.com/XjuSelab/xju-feiyue)

## 初步技术方向

- 前端：React、TypeScript、Vite、Tailwind CSS、shadcn/ui、TanStack Query、Zod
- 后端：Rust、Axum、Tokio、SQLx、PostgreSQL
- 文件：本地对象目录起步，保留 S3/MinIO 兼容接口
- Agent：外部模型 API + 项目结构化数据检索，首期只读
- 部署：4 核 8 GB Linux 服务器，Docker Compose，域名 `icthub.top`

## 计划目录

```text
xju-icthub/
├── frontend/        React + TypeScript
├── backend/         Rust + Axum
├── docs/            开题、架构、需求与部署文档
├── deploy/          Docker Compose 与反向代理配置
└── README.md
```

## License

许可证将在正式进入开发阶段前确认。
