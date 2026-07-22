# 新疆大学 ICT&软开实验室主页

`xju-icthub` 是新疆大学 ICT&软开实验室的统一站点仓库，目标域名为 `icthub.top`。

当前首期建设内容是其中的“项目库”模块，用于统一整理实验室历届与在研项目，记录项目类别、内容简介、曾获奖、负责人、状态与相关资源，并提供快速导入、检索和 Agent 问答能力。

## 站点与模块定位

- `/`：实验室主页，首期保留站点壳与内容占位；
- `/projects`：项目库列表，当前主要开发页面；
- `/projects/:slug`：项目详情；
- `/admin/projects`：项目管理入口；
- 未来可增加成员、研究方向、成果、加入我们等全局导航。

全局顶部导航将作为独立站点壳预留。首期不在项目页面中写死完整实验室导航，避免后续扩展时重构页面层级。

## 当前阶段

项目处于开题与架构设计阶段。项目库默认采用简约的“编辑部列表”：突出项目名称、内容简介、主类别与曾获奖，其他管理字段进入项目详情。

- [项目开题文档](docs/项目开题文档.md)
- [项目库交互原型](docs/prototypes/02-editorial-list.html)
- [前端设计参考：XjuSelab/xju-feiyue](https://github.com/XjuSelab/xju-feiyue)

## 初步技术方向

- 前端：React、TypeScript、Vite、Tailwind CSS、shadcn/ui、TanStack Query、Zod
- 后端：Rust、Axum、Tokio、SQLx、PostgreSQL
- 文件：本地对象目录起步，保留 S3/MinIO 兼容接口
- Agent：外部模型 API + 项目结构化数据检索，首期只读
- 部署：4 核 8 GB Linux 服务器，Docker Compose，域名 `icthub.top`

## 计划目录

```text
xju-icthub/
├── frontend/        实验室站点壳与各业务模块
├── backend/         Rust + Axum API
├── docs/            开题、架构、需求与部署文档
├── deploy/          Docker Compose 与反向代理配置
└── README.md
```

## License

许可证将在正式进入开发阶段前确认。
