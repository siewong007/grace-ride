import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Fingerprint, LogOut, ShieldCheck } from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import { Avatar, ErrorBox, Field, FileInput, Modal, Pill, useAction } from '../components/ui';
import { fmtAgo, ROLE_LABEL, useFormatT } from '../format';
import { passkeySupported, registerPasskey } from '../webauthn';
import { useTranslation } from 'react-i18next';

export default function Profile() {
  const { t, ft } = useTranslationHooks();
  const { me, refresh, has } = useAuth();
  const nav = useNavigate();
  const passkeys = useApi<{ id: number; label: string; created_at: string; last_used: string | null }[]>('/auth/passkeys');
  const [f, setF] = useState({ name: me!.name, phone: me!.phone, photo: null as number | null, optOut: me!.badges_opt_out });
  const [totp, setTotp] = useState<{ secret: string; url: string } | null>(null);
  const { busy, error, run } = useAction();
  if (!me) return null;

  return (
    <div className="stack">
      <div className="page-head">
        <div className="row nowrap">
          <Avatar name={me.name} fileId={me.photo_file_id} size="lg" />
          <div>
            <h1>{me.name}</h1>
            <p>{me.email}</p>
          </div>
        </div>
        <button className="btn" onClick={() => run(async () => { await post('/auth/logout'); await refresh(); nav('/login'); })}><LogOut size={18} />{t('logout')}</button>
      </div>
      <div className="row">
        <Pill tone={me.status === 'pending' ? 'gray' : 'green'}>{me.status === 'pending' ? t('pending') : t('member')}</Pill>
        {me.roles.map((r) => <Pill key={r} tone="blue">{ft(ROLE_LABEL[r])}</Pill>)}
        {me.groups.map((g) => <Pill key={g.id}>{g.name}</Pill>)}
      </div>
      {me.status === 'pending' && <div className="notice">{t('pendingNotice')}</div>}

      {me.status !== 'pending' && <div className="grid cols-2 hide-tablet-up">
        <Link className="btn" to="/costs">{t('myCosts')}</Link>
        <Link className="btn" to="/reimbursements">{t('myReimb')}</Link>
        <Link className="btn" to="/report">{t('myReport')}</Link>
        <Link className="btn" to="/help">{t('myHelp')}</Link>
        {has('leader') && <Link className="btn" to="/settlements">{t('settlements')}</Link>}
        {me.mfa_ok && (has('transport') || has('admin')) && <Link className="btn" to="/admin">{t('dashboard')}</Link>}
        {me.mfa_ok && (has('transport') || has('finance')) && <Link className="btn" to="/admin/reimbursements">{t('reimbReview')}</Link>}
        {me.mfa_ok && has('admin') && <Link className="btn" to="/admin/members">{t('members')}</Link>}
      </div>}

      <div className="split">
        <form className="card stack" onSubmit={(e) => { e.preventDefault(); run(() => put('/me', { name: f.name, phone: f.phone, photo_file_id: f.photo, badges_opt_out: f.optOut }).then(refresh), t('saved')); }}>
          <h3>{t('name')}</h3>
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
          <button className="btn primary" disabled={busy}>{t('save')}</button>
        </form>

        <div className="card stack">
          <h3><ShieldCheck size={20} /> {t('security')}</h3>
          {me.privileged && !me.mfa_ok && <div className="notice">{t('mfaRequired')}</div>}
          <div className="stack-sm">
            <div className="row between card flat tight">
              <div><div className="bold">{t('passkeys')}</div></div>
            </div>
            {passkeys.data?.length ? (
              <div className="list">
                {passkeys.data.map((p) => (
                  <div key={p.id} className="row between card flat tight">
                    <div><div className="bold">{p.label}</div><div className="muted tiny">{t('passkeyLabel', { date: fmtAgo(p.created_at, ft) })}{p.last_used && ` · ${t('lastUsed', { date: fmtAgo(p.last_used, ft) })}`}</div></div>
                    <button className="btn sm danger" onClick={() => confirm(t('removeConfirm')) && run(() => del(`/auth/passkeys/${p.id}`).then(passkeys.reload).then(refresh), t('removed'))}>{t('remove')}</button>
                  </div>
                ))}
              </div>
            ) : <p className="muted">无</p>}
            <button className="btn primary" disabled={busy || !passkeySupported()} onClick={() => run(() => registerPasskey(navigator.platform || t('myPhone')).then(passkeys.reload).then(refresh), t('addPasskeyDone'))}>
              <Fingerprint size={18} />{t('addPasskey')}
            </button>
          </div>
          {me.status !== 'pending' && <>
          <div className="divider" />
          <div className="stack-sm">
            <div className="row between"><strong>{t('totp')}</strong>{me.totp_enabled ? <Pill tone="green">{t('totpEnabled')}</Pill> : <Pill>{t('totpDisabled')}</Pill>}</div>
            <p className="muted small">{t('totpNote')}</p>
            {!me.totp_enabled && <button className="btn" disabled={busy} onClick={() => run(async () => setTotp(await post('/auth/totp/setup')))}>{t('setupTotp')}</button>}
          </div>
          </>}
          <ErrorBox error={error} />
        </div>
      </div>
      {totp && <TotpModal secret={totp.secret} url={totp.url} onClose={() => setTotp(null)} onDone={() => { setTotp(null); refresh(); }} />}
    </div>
  );

  function useTranslationHooks() {
    const { t } = useTranslation('profile');
    const ft = useFormatT();
    return { t, ft };
  }
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
      <code className="card flat tight mono" style={{ wordBreak: 'break-all' }}>{secret}</code>
      <a className="small" href={url}>{t('totpJoin')}</a>
      <Field label={t('totpCode')}>
        <input className="input" inputMode="numeric" maxLength={6} value={code} onChange={(e) => setCode(e.target.value)} />
      </Field>
      <ErrorBox error={error} />
      <button className="btn primary" disabled={busy || code.length !== 6} onClick={() => run(() => post('/auth/totp/enable', { code }).then(onDone), t('enabled'))}>{t('enable')}</button>
    </Modal>
  );
}


