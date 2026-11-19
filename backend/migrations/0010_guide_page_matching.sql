-- Point seeded help-guide steps at the page they are done on, and fold duplicate guides.
--
-- The header "Guide" button offers the guide whose steps cover the current page. Many
-- seeded steps named a list page ("/rides", "/events") for work done on an item's page,
-- so the button opened the wrong guide, or a guide halfway through. Step links may now
-- use `*` for an item ("/rides/*" = any ride's page); the app never navigates to a `*`
-- link, it sends the person to the list instead (frontend/src/guideLinks.ts).
--
-- Safe on any database:
--   • only guides no one has saved in the editor are touched (updated_by IS NULL;
--     the editor always sets it, the seed never does);
--   • each step changes only while its title and link still hold the seeded values;
--   • "Pay reimbursements" duplicated "Review and pay reimbursements", so it is
--     unpublished, not deleted: its feedback stays and an editor can publish it again;
--   • a replay matches nothing, because every guard checks the old value.
-- Fresh databases get the new content from seed.rs and match no row here.

-- 1. Step links.
WITH fixes(slug, idx, title_en, old_link, new_link) AS (
    VALUES
        ('plan-transport',        0, 'Find the event',                '/events',          '/events/*'),
        ('plan-transport',        1, 'Open the planning board',       '',                 '/admin/events/*/plan'),
        ('send-announcement',     1, 'Members get notified',          '/notifications',   '/admin/announcements'),
        ('request-a-ride',        1, 'Link an event',                 '/events',          '/rides/new'),
        ('track-your-ride',       1, 'Coordinate or cancel',          '/rides',           '/rides/*'),
        ('join-events',           1, 'Manage your place',             '/events',          '/events/*'),
        ('organise-an-event',     1, 'Record attendance and finish',  '/events',          '/events/*'),
        ('split-activity-costs',  0, 'Create the bill',               '/events',          '/events/*/settle'),
        ('claim-reimbursement',   1, 'Resubmit a returned claim',     '/reimbursements',  '/reimbursements/*'),
        ('answer-a-driver-offer', 1, 'Accept or decline',             '/rides',           '/rides/*'),
        ('cant-make-it',          1, 'Ready or running late',         '/rides',           '/rides/*'),
        ('cant-make-it',          2, 'Can''t join today',             '/rides',           '/rides/*'),
        ('join-as-a-group',       1, 'Choose who you''re joining as', '/events',          '/events/*'),
        ('join-as-a-group',       2, 'Ask for a ride',                '/events',          '/events/*'),
        ('bring-a-visitor',       1, 'Send the link',                 '/events',          '/events/*'),
        ('bring-a-visitor',       2, 'If something changes',          '/events',          '/events/*'),
        ('manage-any-event',      1, 'Edit or finish',                '/events',          '/events/*')
),
rebuilt AS (
    SELECT g.id,
           jsonb_agg(
               CASE WHEN f.new_link IS NULL THEN s.step
                    ELSE jsonb_set(s.step, '{link}', to_jsonb(f.new_link)) END
               ORDER BY s.ord
           ) AS steps
    FROM guide_workflows g
    CROSS JOIN LATERAL jsonb_array_elements(g.steps) WITH ORDINALITY AS s(step, ord)
    LEFT JOIN fixes f
           ON f.slug = g.slug
          AND f.idx = s.ord - 1
          AND f.title_en = s.step ->> 'title_en'
          AND f.old_link = COALESCE(s.step ->> 'link', '')
    WHERE g.updated_by IS NULL
      AND g.id IN (
        SELECT g2.id
        FROM guide_workflows g2
        CROSS JOIN LATERAL jsonb_array_elements(g2.steps) WITH ORDINALITY AS s2(step, ord)
        JOIN fixes f2
          ON f2.slug = g2.slug
         AND f2.idx = s2.ord - 1
         AND f2.title_en = s2.step ->> 'title_en'
         AND f2.old_link = COALESCE(s2.step ->> 'link', '')
    )
    GROUP BY g.id
)
UPDATE guide_workflows g
SET steps = rebuilt.steps
FROM rebuilt
WHERE g.id = rebuilt.id;

-- 2. "Manage members and invites" → "Manage members": invites and recovery have their
--    own guides ("Invites", "Account recovery"), so the overview drops that step.
UPDATE guide_workflows
SET title_en   = 'Manage members',
    title_zh   = '管理会员',
    summary_en = 'Approve members, set groups, and disable or restore accounts.',
    summary_zh = '审批新会员、设置小组、停用或恢复账号。',
    steps      = jsonb_build_array(steps -> 0)
WHERE slug = 'manage-members'
  AND updated_by IS NULL
  AND title_en = 'Manage members and invites'
  AND jsonb_array_length(steps) = 2
  AND steps -> 1 ->> 'title_en' = 'Invites and recovery';

-- 3. "Audit and settings" → "Read the audit log": settings have their own guide
--    ("Groups, areas and rates").
UPDATE guide_workflows
SET title_en   = 'Read the audit log',
    title_zh   = '查看审计日志',
    summary_en = 'See who did what, filter it and export it.',
    summary_zh = '查看谁做了什么，可筛选并导出。',
    steps      = jsonb_build_array(steps -> 0)
WHERE slug = 'audit-settings'
  AND updated_by IS NULL
  AND title_en = 'Audit and settings'
  AND jsonb_array_length(steps) = 2
  AND steps -> 1 ->> 'title_en' = 'Settings';

-- 4. "Pay reimbursements" repeated step 2 of "Review and pay reimbursements".
UPDATE guide_workflows
SET published = FALSE
WHERE slug = 'pay-reimbursements'
  AND updated_by IS NULL
  AND published
  AND title_en = 'Pay reimbursements';
