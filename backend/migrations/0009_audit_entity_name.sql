-- Denormalize entity_name on audit_logs so search and listing do not execute
-- 16 correlated subqueries across historical rows.
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS entity_name TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs (created_at);

UPDATE audit_logs a
SET entity_name = COALESCE(
    CASE a.entity
        WHEN 'user' THEN (SELECT name FROM users WHERE id = a.entity_id)
        WHEN 'ride' THEN (SELECT p.name FROM rides r JOIN users p ON p.id = r.passenger_id WHERE r.id = a.entity_id)
        WHEN 'event' THEN (SELECT title FROM events WHERE id = a.entity_id)
        WHEN 'group' THEN (SELECT name FROM groups WHERE id = a.entity_id)
        WHEN 'area' THEN (SELECT name FROM areas WHERE id = a.entity_id)
        WHEN 'settlement' THEN (SELECT title FROM settlements WHERE id = a.entity_id)
        WHEN 'settlement_share' THEN (SELECT p.name FROM settlement_shares s JOIN users p ON p.id = s.user_id WHERE s.id = a.entity_id)
        WHEN 'reimbursement' THEN (SELECT p.name FROM reimbursements r JOIN users p ON p.id = r.user_id WHERE r.id = a.entity_id)
        WHEN 'announcement' THEN (SELECT title FROM announcements WHERE id = a.entity_id)
        WHEN 'help_ticket' THEN (SELECT subject FROM help_tickets WHERE id = a.entity_id)
        WHEN 'file' THEN (SELECT NULLIF(original_name, '') FROM files WHERE id = a.entity_id)
        WHEN 'invite' THEN (SELECT COALESCE(NULLIF(name, ''), NULLIF(email, '')) FROM invites WHERE id = a.entity_id)
        WHEN 'recovery' THEN (SELECT email FROM recovery_requests WHERE id = a.entity_id)
        WHEN 'visitor_pass' THEN (SELECT name FROM visitor_passes WHERE id = a.entity_id)
        WHEN 'guide_workflow' THEN (SELECT COALESCE(NULLIF(title_zh, ''), title_en) FROM guide_workflows WHERE id = a.entity_id)
        WHEN 'role' THEN CASE
            WHEN a.action = 'role.update' THEN COALESCE(
                (SELECT NULLIF(name, '') FROM roles WHERE key = split_part(a.detail, ':', 1)),
                NULLIF(split_part(a.detail, ':', 1), ''))
            ELSE substring(a.detail from '^(.*) \([a-z][a-z0-9_]*\)[:;]')
        END
    END,
    CASE
        WHEN a.action IN ('group.create', 'group.update', 'group.delete', 'area.create',
                          'announcement.create', 'announcement.update', 'announcement.delete',
                          'settlement.create', 'event.create', 'event.cancel')
            THEN NULLIF(a.detail, '')
        WHEN a.action = 'settlement.delete' THEN NULLIF(regexp_replace(a.detail, '^draft: ', ''), '')
    END,
    ''
)
WHERE a.entity_name = '';
