ALTER TABLE projects ADD COLUMN classification_status TEXT NOT NULL DEFAULT 'classified';
ALTER TABLE projects ADD COLUMN legacy_primary_category TEXT;
ALTER TABLE projects ADD COLUMN cover_mode TEXT NOT NULL DEFAULT 'text';
ALTER TABLE projects ADD COLUMN cover_resource_id TEXT;
ALTER TABLE projects ADD COLUMN cover_title TEXT;
ALTER TABLE projects ADD COLUMN cover_subtitle TEXT;
ALTER TABLE projects ADD COLUMN cover_keywords TEXT NOT NULL DEFAULT '[]';
ALTER TABLE projects ADD COLUMN cover_tone TEXT NOT NULL DEFAULT 'slate';
ALTER TABLE projects ADD COLUMN cover_confidence REAL;
ALTER TABLE projects ADD COLUMN cover_generated_at TEXT;

ALTER TABLE project_tags ADD COLUMN tag_definition_id TEXT;

CREATE TABLE IF NOT EXISTS tag_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    group_name TEXT NOT NULL DEFAULT '其他',
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    merged_into_id TEXT REFERENCES tag_definitions(id),
    created_by_sid TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tag_definitions_group_sort
    ON tag_definitions(group_name, sort_order, name);

CREATE TABLE IF NOT EXISTS tag_suggestions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    group_name TEXT,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_by_sid TEXT NOT NULL,
    reviewed_by_sid TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO tag_definitions (id, name, group_name, color, sort_order) VALUES
    ('tag-competition-innovation', '国创赛（互联网+）', '比赛', '#b8405e', 10),
    ('tag-competition-design', '计算机设计大赛', '比赛', '#0b6e99', 20),
    ('tag-competition-intelligent', '智能应用技术大赛', '比赛', '#9065b0', 30),
    ('tag-tech-big-data', '大数据', '技术', '#9065b0', 110),
    ('tag-tech-ai-application', '人工智能应用', '技术', '#9065b0', 120),
    ('tag-tech-llm-agent', 'LLM/Agent', '技术', '#9065b0', 130),
    ('tag-tech-cv', '计算机视觉', '技术', '#9065b0', 140),
    ('tag-tech-nlp', 'NLP', '技术', '#9065b0', 150),
    ('tag-tech-iot', '物联网', '技术', '#b7791f', 160),
    ('tag-tech-embedded', '嵌入式', '技术', '#b7791f', 170),
    ('tag-tech-robot', '机器人', '技术', '#b7791f', 180),
    ('tag-tech-web', 'Web', '技术', '#0f7b6c', 190),
    ('tag-tech-mobile', '移动端', '技术', '#0f7b6c', 200),
    ('tag-tech-3d-vr', '3D/VR', '技术', '#0b6e99', 210),
    ('tag-feature-hybrid', '软硬结合', '特征', '#b7791f', 310),
    ('tag-feature-ai-core', 'AI核心', '特征', '#9065b0', 320),
    ('tag-feature-ai-enhanced', 'AI增强', '特征', '#9065b0', 330),
    ('tag-feature-non-ai', '非AI', '特征', '#64748b', 340),
    ('tag-feature-open-source', '开源项目', '特征', '#0f7b6c', 350),
    ('tag-domain-campus', '校园服务', '领域', '#0f7b6c', 410),
    ('tag-domain-education', '教育', '领域', '#0f7b6c', 420),
    ('tag-domain-agriculture', '农业', '领域', '#0f7b6c', 430),
    ('tag-domain-medical', '医疗', '领域', '#0f7b6c', 440),
    ('tag-domain-tourism', '文旅', '领域', '#0f7b6c', 450),
    ('tag-domain-industry', '工业', '领域', '#0f7b6c', 460),
    ('tag-domain-research', '科研辅助', '领域', '#0f7b6c', 470),
    ('tag-source-competition', '比赛项目', '来源', '#64748b', 510),
    ('tag-source-lab', '实验室建设', '来源', '#64748b', 520),
    ('tag-source-course', '课程项目', '来源', '#64748b', 530),
    ('tag-source-tool', '日常工具', '来源', '#64748b', 540),
    ('tag-source-personal', '个人探索', '来源', '#64748b', 550),
    ('tag-source-service', '对外服务', '来源', '#64748b', 560);

INSERT OR IGNORE INTO project_tags (project_id, tag, sort_order, tag_definition_id)
SELECT id, '国创赛（互联网+）', 0, 'tag-competition-innovation'
FROM projects WHERE primary_category = '互联网+';

INSERT OR IGNORE INTO project_tags (project_id, tag, sort_order, tag_definition_id)
SELECT id, '计算机设计大赛', 0, 'tag-competition-design'
FROM projects WHERE primary_category = '计算机设计大赛';

UPDATE projects SET legacy_primary_category = primary_category
WHERE primary_category IN ('互联网+', '计算机设计大赛', '论文', '工具项目', '其他');

UPDATE projects SET primary_category = '研究成果' WHERE primary_category = '论文';
UPDATE projects SET primary_category = '传统软件' WHERE primary_category = '工具项目';

UPDATE projects
SET primary_category = CASE
    WHEN name || summary LIKE '%机器人%' OR name || summary LIKE '%嵌入式%'
        OR name || summary LIKE '%物联网%' OR name || summary LIKE '%硬件%' THEN '智能硬件'
    WHEN name || summary LIKE '%3D%' OR name || summary LIKE '%VR%'
        OR name || summary LIKE '%游戏%' OR name || summary LIKE '%动画%'
        OR name || summary LIKE '%数字展%' OR name || summary LIKE '%交互设计%' THEN '数字媒体'
    WHEN name || summary LIKE '%论文%' OR name || summary LIKE '%实验研究%'
        OR name || summary LIKE '%方法研究%' OR name || summary LIKE '%调研报告%' THEN '研究成果'
    WHEN name || summary LIKE '%AI%' OR name || summary LIKE '%人工智能%'
        OR name || summary LIKE '%模型%' OR name || summary LIKE '%算法%'
        OR name || summary LIKE '%预测%' OR name || summary LIKE '%识别%'
        OR name || summary LIKE '%大数据%' THEN 'AI 软件'
    ELSE '传统软件'
END
WHERE primary_category IN ('互联网+', '计算机设计大赛');

UPDATE projects
SET primary_category = '传统软件', classification_status = 'pending'
WHERE primary_category = '其他';

INSERT OR IGNORE INTO tag_definitions (id, name, group_name, sort_order, created_by_sid)
SELECT 'legacy-' || lower(hex(randomblob(16))), tag, '历史', 900, 'migration'
FROM project_tags;

UPDATE project_tags
SET tag_definition_id = (
    SELECT id FROM tag_definitions WHERE tag_definitions.name = project_tags.tag COLLATE NOCASE
)
WHERE tag_definition_id IS NULL;

