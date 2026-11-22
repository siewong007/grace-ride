import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api, del, post, useApi } from '../../api';
import { useAuth } from '../../auth';
import { Button, ErrorBox, Field, Pill, useAction, useConfirm } from '../../components/ui';

type Member = { user_id: number; name: string; status: string };
type RequestRow = { id: number; user_id: number; name: string };
type InviteRow = { id: number; phone: string; expires_at: string; url: string };
type Incoming = { id: number; family_name: string; owner_name: string };

type FamilyView = {
  family: { id: number; name: string; owner_id: number } | null;
  members: Member[];
  requests: RequestRow[];
  invites: InviteRow[];
  incoming: Incoming | null;
};

export default function FamilySection() {
  const { t } = useTranslation('profile');
  const { me } = useAuth();
  const confirm = useConfirm();
  const { data, reload } = useApi<FamilyView>('/me/family');
  const { busy, error, run } = useAction();
  const [name, setName] = useState('');
  const [rename, setRename] = useState('');
  const [phone, setPhone] = useState('');

  if (!me || !data) return null;

  const family = data.family;
  const owner = family != null && family.owner_id === me.id;
  const done = () => reload();

  if (!family && data.incoming) {
    const incoming = data.incoming;
    return (
      <div className="stack">
        <p>
          {incoming.owner_name} · {incoming.family_name}
        </p>
        <div className="btn-row">
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => run(() => post(`/me/family/requests/${incoming.id}/accept`).then(done))}
          >
            {t('accept')}
          </Button>
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => run(() => post(`/me/family/requests/${incoming.id}/decline`).then(done))}
          >
            {t('decline')}
          </Button>
        </div>
        <ErrorBox error={error} />
      </div>
    );
  }

  if (!family) {
    if (me.status !== 'active') {
      return <p className="muted">{t('familyPendingHint')}</p>;
    }
    return (
      <form
        className="stack"
        onSubmit={(e) => {
          e.preventDefault();
          run(() => post('/me/family', { name: name.trim() }).then(done));
        }}
      >
        <Field label={t('familyName')}>
          <input className="input" required value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Button variant="primary" type="submit" disabled={busy}>
          {t('createFamily')}
        </Button>
        <ErrorBox error={error} />
      </form>
    );
  }

  return (
    <div className="stack">
      <h2>{family.name}</h2>
      <ul className="stack-xs">
        {data.members.map((m) => (
          <li key={m.user_id} data-testid="family-member" className="row between align-center">
            <span>{m.name}</span>
            <span className="row gap-sm">
              {m.status === 'pending' && <Pill tone="warning">{t('waitingApproval')}</Pill>}
              {owner && m.user_id !== me.id && (
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={busy}
                  onClick={async () => {
                    const ok = await confirm({
                      title: t('removeMember'),
                      body: t('removeMemberConfirm'),
                      confirmLabel: t('removeMember'),
                      tone: 'danger',
                    });
                    if (ok) run(() => del(`/me/family/members/${m.user_id}`).then(done));
                  }}
                >
                  {t('removeMember')}
                </Button>
              )}
            </span>
          </li>
        ))}
      </ul>

      {owner && (
        <>
          <form
            className="stack"
            onSubmit={(e) => {
              e.preventDefault();
              run(() => api('/me/family', { method: 'PATCH', body: { name: rename.trim() } }).then(done));
            }}
          >
            <Field label={t('familyName')}>
              <input className="input" required value={rename} onChange={(e) => setRename(e.target.value)} />
            </Field>
            <Button variant="secondary" type="submit" disabled={busy}>
              {t('renameFamily')}
            </Button>
          </form>

          <form
            className="stack"
            onSubmit={(e) => {
              e.preventDefault();
              run(() => post('/me/family/people', { phone: phone.trim() }).then(() => {
                setPhone('');
                return done();
              }));
            }}
          >
            <Field label={t('addPhone')}>
              <input className="input" type="tel" required value={phone} onChange={(e) => setPhone(e.target.value)} />
            </Field>
            <Button variant="primary" type="submit" disabled={busy}>
              {t('addPerson')}
            </Button>
          </form>

          {data.requests.length > 0 && (
            <ul className="stack-xs">
              {data.requests.map((r) => (
                <li key={r.id} className="row between">
                  <span>{r.name}</span>
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => run(() => del(`/me/family/requests/${r.id}`).then(done))}
                  >
                    {t('cancelRequest')}
                  </Button>
                </li>
              ))}
            </ul>
          )}

          {data.invites.length > 0 && (
            <ul className="stack-xs">
              {data.invites.map((inv) => (
                <li key={inv.id} className="row between align-center">
                  <span>{inv.phone}</span>
                  <span className="row gap-sm">
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => navigator.clipboard.writeText(inv.url)}
                    >
                      {t('copyLink')}
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busy}
                      onClick={() => run(() => del(`/me/family/invites/${inv.id}`).then(done))}
                    >
                      {t('cancelInvite')}
                    </Button>
                  </span>
                </li>
              ))}
            </ul>
          )}

          <Button
            variant="secondary"
            disabled={busy}
            onClick={async () => {
              const ok = await confirm({
                title: t('deleteFamily'),
                body: t('deleteFamilyConfirm'),
                confirmLabel: t('deleteFamily'),
                tone: 'danger',
              });
              if (ok) run(() => del('/me/family').then(done));
            }}
          >
            {t('deleteFamily')}
          </Button>
        </>
      )}

      {!owner && (
        <Button
          variant="secondary"
          disabled={busy}
          onClick={async () => {
            const ok = await confirm({
              title: t('leaveFamily'),
              body: t('leaveFamilyConfirm'),
              confirmLabel: t('leaveFamily'),
              tone: 'danger',
            });
            if (ok) run(() => post('/me/family/leave').then(done));
          }}
        >
          {t('leaveFamily')}
        </Button>
      )}
      <ErrorBox error={error} />
    </div>
  );
}
