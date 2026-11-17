-- Fix seeded help-guide step links that opened a page that cannot load.
--
-- Some seeded steps linked to route patterns instead of pages ("/rides/:id",
-- "/events/:eventId/settle", ...). Following one opened the detail page for the
-- literal id ":id", and the API answered "Not found". One step pointed at the example
-- invite "/invite/example", which fails the same way. Each now points at the list or
-- landing page where the person picks the real item.
--
-- Safe on any database:
--   • the API refuses a step link containing ':' (validate_guide), so every row this
--     touches is untouched seed data, never an editor's work;
--   • only the listed (guide, link) pairs change, and step order is kept;
--   • it matches nothing once the links are fixed, so a replay changes no row.
-- Fresh databases get the corrected links from seed.rs and match no row here.

WITH fixes(slug, old_link, new_link) AS (
    VALUES
        ('getting-started',    '/invite/example',         ''),
        ('track-your-ride',     '/rides/:id',              '/rides'),
        ('volunteer-driving',   '/rides/:id',              '/drive'),
        ('join-events',         '/events/:id',             '/events'),
        ('organise-an-event',   '/events/:id',             '/events'),
        ('split-activity-costs', '/events/:id/settle',     '/events'),
        ('pay-your-share',      '/settlements/:id',        '/costs'),
        ('ask-for-help',        '/help/:id',               '/help'),
        ('manage-rides',        '/admin/rides/:id',        '/admin/rides'),
        ('manage-any-event',    '/events/:id',             '/events')
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
    LEFT JOIN fixes f ON f.slug = g.slug AND f.old_link = s.step ->> 'link'
    WHERE g.id IN (
        SELECT g2.id
        FROM guide_workflows g2
        CROSS JOIN LATERAL jsonb_array_elements(g2.steps) AS s2(step)
        JOIN fixes f2 ON f2.slug = g2.slug AND f2.old_link = s2.step ->> 'link'
    )
    GROUP BY g.id
)
UPDATE guide_workflows g
SET steps = rebuilt.steps
FROM rebuilt
WHERE g.id = rebuilt.id;
