-- Expand mobile_push_tokens.provider CHECK constraint to permit Apple APNs ('apns')
-- tokens registered by the native Swift iOS client alongside 'fcm' and 'unifiedpush'.
ALTER TABLE mobile_push_tokens DROP CONSTRAINT IF EXISTS chk_mobile_push_provider;
ALTER TABLE mobile_push_tokens ADD CONSTRAINT chk_mobile_push_provider
    CHECK (provider IN ('fcm', 'unifiedpush', 'apns'));
