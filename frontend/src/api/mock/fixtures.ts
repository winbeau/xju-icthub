import type { ProjectDetail } from '@/api/schemas/project'

const cover = {
  coverMode: 'text' as const,
  coverResourceId: null,
  coverResourceUrl: null,
  coverConfidence: 0.74,
}

export const PROJECT_FIXTURES: readonly ProjectDetail[] = [
  {
    id: '1', slug: 'multimodal-inspection-robot', primaryCategory: '智能硬件', classificationStatus: 'classified',
    name: '多模态巡检机器人', summary: '面向办公空间和机房的视觉巡检、异常识别与远程处置系统。',
    highestAward: '国创赛省赛银奖', status: '研发中', critique: '现场稳定性决定它能否真正投入使用。',
    ownerName: '机器人项目组', sourceName: '2023 届项目组', tags: ['机器人', '计算机视觉', '国创赛（互联网+）'],
    resources: [
      { id: 'r1', type: 'github', title: '项目代码仓库', url: 'https://github.com/example/robot' },
      { id: 'r1-doc', type: 'document', title: '项目设计说明书', url: 'https://example.com/robot-document.pdf' },
      { id: 'r1-ppt', type: 'presentation', title: '国创赛答辩 PPT', url: 'https://example.com/robot-presentation.pptx' },
      { id: 'r1-video', type: 'video', title: '巡检流程展示视频', url: 'https://example.com/robot-demo.mp4' },
      { id: 'r1-poster', type: 'image', title: '项目展示海报', url: 'https://example.com/robot-poster.png' },
    ],
    ...cover, coverTitle: '智能巡检', coverSubtitle: '机房异常识别与远程处置', coverKeywords: ['机器人', '计算机视觉', '物联网'], coverTone: 'amber',
  },
  {
    id: '2', slug: 'campus-energy-forecast', primaryCategory: 'AI 软件', classificationStatus: 'classified',
    name: '校园能耗预测平台', summary: '利用历史能耗和天气数据预测楼宇用电，并提供调度建议。',
    highestAward: '计算机设计大赛省级一等奖', status: '运维测试', critique: '继续聚焦可解释预测和数据漂移。',
    ownerName: '数据智能组', sourceName: '2024 计算机设计大赛团队', tags: ['大数据', '校园服务', '计算机设计大赛'],
    resources: [{ id: 'r2', type: 'document', title: '实验报告', url: null }],
    ...cover, coverTitle: '智能预测', coverSubtitle: '校园楼宇能耗分析与调度', coverKeywords: ['大数据', '校园服务', '人工智能应用'], coverTone: 'violet',
  },
  {
    id: '3', slug: 'edge-object-detection', primaryCategory: '研究成果', classificationStatus: 'classified',
    name: '面向边缘设备的轻量目标检测研究', summary: '研究剪枝、量化与蒸馏对边缘端检测精度和延迟的影响。',
    highestAward: '校级优秀论文', status: '迁移中', critique: '需要用跨设备实验验证方法的普适性。',
    ownerName: '边缘智能组', sourceName: '学年论文项目', tags: ['计算机视觉', '嵌入式', '科研辅助'],
    resources: [{ id: 'r3', type: 'document', title: '论文初稿', url: null }],
    ...cover, coverTitle: '边缘检测', coverSubtitle: '轻量模型精度与延迟研究', coverKeywords: ['计算机视觉', '嵌入式', '科研辅助'], coverTone: 'emerald',
  },
  {
    id: '4', slug: 'device-booking', primaryCategory: '传统软件', classificationStatus: 'classified',
    name: '设备预约管理系统', summary: '管理设备档案、预约冲突、借用交接和二维码盘点。',
    highestAward: null, status: '研发中', critique: '清晰的责任链就是最重要的体验。',
    ownerName: '个人维护', sourceName: '实际需求', tags: ['Web', '日常工具', '个人探索'],
    resources: [{ id: 'r4', type: 'github', title: 'GitHub 仓库', url: 'https://github.com/example/booking' }],
    ...cover, coverTitle: '便捷预约', coverSubtitle: '设备借用与盘点', coverKeywords: ['Web', '日常工具', '个人探索'], coverTone: 'slate',
  },
  {
    id: '5', slug: 'intangible-heritage-showcase', primaryCategory: '数字媒体', classificationStatus: 'classified',
    name: '非遗数字交互展示平台', summary: '通过三维交互和叙事设计展示非遗技艺与文化脉络。',
    highestAward: '计算机设计大赛国赛二等奖', status: '已归档', critique: '让交互真正服务内容表达。',
    ownerName: '数字创意组', sourceName: '2022 届项目组', tags: ['3D/VR', '文旅', '计算机设计大赛'],
    resources: [{ id: 'r5', type: 'video', title: '作品视频', url: null }],
    ...cover, coverTitle: '非遗新境', coverSubtitle: '三维交互讲述文化脉络', coverKeywords: ['3D/VR', '文旅', '计算机设计大赛'], coverTone: 'cyan',
  },
]
