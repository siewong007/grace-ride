import { ReactNode, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ChevronRight, Clock, Fingerprint, LogOut, ShieldAlert, ShieldCheck } from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { DisplayPreferences } from '../components/AppChrome';
import { Button, Card, ErrorBox, Field, FileInput, Modal, Notice, PageHeader, Pill, useAction, useConfirm } from '../components/ui';
import { fmtAgo, roleLabel, useFormatT } from '../format';
import { passkeySupported, registerPasskey } from '../webauthn';
import { useTranslation } from 'react-i18next';

/** A row in the phone-only shortcut list: label and a chevron. */
function ProfileLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Link to={to} className="item profile-link">
      <span className="grow">{children}</span>
      <ChevronRight size={18} className="muted" aria-hidden="true" />
    </Link>
  );
}

export default function Profile() {
  const { t } = useTranslation('profile');
  const ft = useFormatT();
  const confirm = useConfirm();
  const { me, refresh, can } = useAuth();
  const nav = useNavigate();
  const passkeys = useApi<{ id: number; label: string; created_at: string; last_used: string | null }[]>('/auth/passkeys');
  const [f, setF] = useState({ name: me?.name ?? '', phone: me?.phone ?? '', photo: null as number | null, optOut: me?.badges_opt_out ?? false });
  const [totp, setTotp] = useState<{ secret: string; url: string } | null>(null);
  const { busy, error, run } = useAction();
  if (!me) return null;

  return (
    <div className="stack">
      <PageHeader title={me.name} subtitle={me.email} />

      {me.status === 'pending' && (
        <Notice tone="warning" icon={<Clock size={20} />}>
          {t('pendingNotice')}
        </Notice>
      )}
      {me.privileged && !me.mfa_ok && (
        <Notice tone="info" icon={<ShieldAlert size={20} />}>
          {t('mfaRequired')}
        </Notice>
      )}

      <div className="profile-grid">
        {/* Section 1: Personal info */}
        <Card className="stack">
          <h3>{t('personalInfo')}</h3>
          <form
            className="stack"
            onSubmit={(e) => {
              e.preventDefault();
              run(() => put('/me', { name: f.name, phone: f.phone, photo_file_id: f.photo, badges_opt_out: f.optOut }).then(refresh), t('saved'));
            }}
          >
            <Field label={t('name')}>
              <input className="input" required value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} />
            </Field>
            <Field label={t('phone')} hint={t('phoneHint')}>
              <input className="input" type="tel" value={f.phone} onChange={(e) => setF({ ...f, phone: e.target.value })} />
            </Field>
            <FileInput label={t('photo')} accept="image/*" value={f.photo} onChange={(id) => setF({ ...f, photo: id })} />
            <label className={`check ${f.optOut ? 'on' : ''}`}>
              <input type="checkbox" checked={f.optOut} onChange={(e) => setF({ ...f, optOut: e.target.checked })} /> {t('optOut')}
            </label>
            <Button variant="primary" type="submit" loading={busy}>{t('save')}</Button>
          </form>
        </Card>

        {/* Section 2: Display */}
        <Card className="stack">
          <h3>{t('display')}</h3>
          <DisplayPreferences />
        </Card>

        {/* Section 3: Security */}
        <Card className="stack">
          <h3><ShieldCheck size={20} /> {t('security')}</h3>
          <div className="stack-sm">
            <div className="bold">{t('passkeys')}</div>
            {passkeys.data?.length ? (
              <div className="passkey-list stack-sm">
                {passkeys.data.map((p) => (
                  <div key={p.id} className="passkey-row row between">
                    <div>
                      <div className="bold">{p.label}</div>
                      <div className="muted tiny">
                        {t('passkeyLabel', { date: fmtAgo(p.created_at, ft) })}
                        {p.last_used && ` · ${t('lastUsed', { date: fmtAgo(p.last_used, ft) })}`}
                      </div>
                    </div>
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={async () => {
                        const ok = await confirm({
                          title: t('remove'),
                          body: t('removeConfirm'),
                          confirmLabel: t('remove'),
                          tone: 'danger',
                        });
                        if (ok) run(() => del(`/auth/passkeys/${p.id}`).then(passkeys.reload).then(refresh), t('removed'));
                      }}
                    >
                      {t('remove')}
                    </Button>
                  </div>
                ))}
              </div>
            ) : (
              <p className="muted">{t('none')}</p>
            )}
            <Button
              variant="primary"
              disabled={busy || !passkeySupported()}
              onClick={() => run(() => registerPasskey(navigator.platform || t('myPhone')).then(passkeys.reload).then(refresh), t('addPasskeyDone'))}
            >
              <Fingerprint size={18} />
              {t('addPasskey')}
            </Button>
          </div>

          {me.status !== 'pending' && (
            <>
              <div className="divider" />
              <div className="stack-sm">
                <div className="row between">
                  <strong>{t('totp')}</strong>
                  <Pill tone={me.totp_enabled ? 'success' : 'neutral'}>
                    {me.totp_enabled ? t('totpEnabled') : t('totpDisabled')}
                  </Pill>
                </div>
                <p className="muted small">{t('totpNote')}</p>
                {!me.totp_enabled && (
                  <Button
                    variant="secondary"
                    disabled={busy}
                    onClick={() => run(async () => setTotp(await post('/auth/totp/setup')))}
                  >
                    {t('setupTotp')}
                  </Button>
                )}
              </div>
            </>
          )}
          <ErrorBox error={error} />
        </Card>

        {/* Section 4: Account */}
        <Card className="stack">
          <h3>{t('account')}</h3>
          <div className="stack-sm">
            <div className="bold small">{t('status')}</div>
            <div className="row">
              <Pill tone={me.status === 'pending' ? 'warning' : 'success'}>
                {me.status === 'pending' ? t('pending') : t('member')}
              </Pill>
              {me.roles.map((r) => <Pill key={r} tone="primary">{roleLabel(r, ft, me.role_names)}</Pill>)}
              {me.groups.map((g) => <Pill key={g.id}>{g.name}</Pill>)}
            </div>
          </div>

          <div className="divider" />

          {me.status !== 'pending' && (
            <div className="stack-sm hide-tablet-up">
              <div className="bold small">{t('roles')}</div>
              <nav className="list profile-links" aria-label={t('roles')}>
                {can('costs') && <ProfileLink to="/costs">{t('myCosts')}</ProfileLink>}
                {can('reimbursements') && <ProfileLink to="/reimbursements">{t('myReimb')}</ProfileLink>}
                {can('report') && <ProfileLink to="/report">{t('myReport')}</ProfileLink>}
                {can('help') && <ProfileLink to="/help">{t('myHelp')}</ProfileLink>}
                {(can('settlements') || can('admin_bills')) && <ProfileLink to="/settlements">{t('settlements')}</ProfileLink>}
                {can('admin_dashboard') && <ProfileLink to="/admin">{t('dashboard')}</ProfileLink>}
                {(can('admin_reimb_review') || can('admin_reimb_pay')) && <ProfileLink to="/admin/reimbursements">{t('reimbReview')}</ProfileLink>}
                {(can('admin_members') || can('admin_invites') || can('admin_recovery')) && <ProfileLink to="/admin/members">{t('members')}</ProfileLink>}
                {can('admin_roles') && <ProfileLink to="/admin/roles">{t('rolesPage')}</ProfileLink>}
              </nav>
              <div className="divider" />
            </div>
          )}

          <div>
            <Button
              variant="secondary"
              onClick={() => run(async () => { await post('/auth/logout'); await refresh(); nav('/login'); })}
            >
              <LogOut size={18} />
              {t('logout')}
            </Button>
          </div>
        </Card>
      </div>

      {totp && <TotpModal secret={totp.secret} url={totp.url} onClose={() => setTotp(null)} onDone={() => { setTotp(null); refresh(); }} />}
    </div>
  );
}

function TotpModal({ secret, url, onClose, onDone }: { secret: string; url: string; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation('profile');
  const [code, setCode] = useState('');
  const { busy, error, run } = useAction();
  return (
    <Modal title={t('totpTitle')} onClose={onClose}>
      <ol className="steps">
        <li>{t('totpStep1')}</li>
        <li>{t('totpStep2')}</li>
      </ol>
      <code className="card flat tight mono break-all">{secret}</code>
      <a className="small" href={url}>{t('totpJoin')}</a>
      <Field label={t('totpCode')}>
        <input className="input" inputMode="numeric" maxLength={6} value={code} onChange={(e) => setCode(e.target.value)} />
      </Field>
      <ErrorBox error={error} />
      <Button variant="primary" disabled={busy || code.length !== 6} onClick={() => run(() => post('/auth/totp/enable', { code }).then(onDone), t('enabled'))}>{t('enable')}</Button>
    </Modal>
  );
}
