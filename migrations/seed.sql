INSERT INTO projects (id, name, description, position) VALUES
    (1, 'Cloudnote', 'Collaborative notes and sync platform', 0),
    (2, 'Shipfast', 'Checkout and fulfillment product', 1),
    (3, 'Northstar API', 'Public developer API', 2);

INSERT INTO assets (project_id, kind, name, detail, status_detail, environment, health, position) VALUES
    (1, 'website', 'app.cloudnote.io', '生产站点 · Vercel', '刚刚检查', '生产', 'healthy', 0),
    (1, 'domain', 'cloudnote.io', 'Cloudflare · 自动续费', '2027-08-16 到期', '生产', 'healthy', 1),
    (1, 'certificate', '*.cloudnote.io', 'Let''s Encrypt · 自动续期', '12 天后续期', '生产', 'warning', 2),
    (1, 'server', 'cn-prod-01', '东京 · 2 vCPU / 4 GB', 'CPU 34% · 运行 128 天', '生产', 'healthy', 3),
    (1, 'service', 'notes-api', 'Docker · :8080', 'v2.8.1 · 3 个实例', '生产', 'healthy', 4),
    (1, 'service', 'sync-worker', 'Docker · 队列任务', '积压 1,248 项', '生产', 'critical', 5),
    (2, 'website', 'shipfa.st', '营销站点 · Netlify', '5 分钟前检查', '生产', 'healthy', 0),
    (2, 'domain', 'shipfa.st', 'Namecheap · 手动续费', '31 天后到期', '生产', 'warning', 1),
    (2, 'server', 'sf-prod-eu', '法兰克福 · 4 vCPU / 8 GB', 'CPU 51% · 运行 46 天', '生产', 'healthy', 2),
    (2, 'service', 'checkout-api', 'systemd · :9000', 'v1.14.0 · 正常', '生产', 'healthy', 3),
    (3, 'domain', 'northstar.dev', 'Cloudflare · 自动续费', '2027-11-04 到期', '生产', 'healthy', 0),
    (3, 'certificate', 'api.northstar.dev', 'Google Trust Services', '67 天后续期', '生产', 'healthy', 1),
    (3, 'server', 'ns-edge-01', '新加坡 · 2 vCPU / 2 GB', 'CPU 18% · 运行 19 天', '生产', 'healthy', 2),
    (3, 'service', 'gateway', 'Docker · :443', 'v4.2.0 · 正常', '生产', 'healthy', 3);
