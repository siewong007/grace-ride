// Minimal WebAuthn helpers translating between webauthn-rs JSON (base64url) and the browser API.
import { post } from './api';

const b64urlToBuf = (s: string): ArrayBuffer => {
  const pad = '='.repeat((4 - (s.length % 4)) % 4);
  const bin = atob((s + pad).replace(/-/g, '+').replace(/_/g, '/'));
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
};

const bufToB64url = (buf: ArrayBuffer | null | undefined): string | undefined => {
  if (!buf) return undefined;
  const bytes = new Uint8Array(buf);
  let bin = '';
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
};

export const passkeySupported = () => typeof window !== 'undefined' && !!window.PublicKeyCredential;

export async function registerPasskey(label?: string) {
  const { challenge_id, options } = await post<{ challenge_id: string; options: any }>('/auth/passkey/register/start');
  const pk = options.publicKey;
  const cred = (await navigator.credentials.create({
    publicKey: {
      ...pk,
      challenge: b64urlToBuf(pk.challenge),
      user: { ...pk.user, id: b64urlToBuf(pk.user.id) },
      excludeCredentials: (pk.excludeCredentials ?? []).map((c: any) => ({ ...c, id: b64urlToBuf(c.id) })),
    },
  })) as PublicKeyCredential | null;
  if (!cred) throw new Error('已取消');
  const r = cred.response as AuthenticatorAttestationResponse;
  const credential = {
    id: cred.id,
    rawId: bufToB64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: bufToB64url(r.attestationObject),
      clientDataJSON: bufToB64url(r.clientDataJSON),
      transports: typeof r.getTransports === 'function' ? r.getTransports() : undefined,
    },
    extensions: cred.getClientExtensionResults(),
  };
  await post('/auth/passkey/register/finish', { challenge_id, credential, label });
}

export async function loginWithPasskey(email: string) {
  const { challenge_id, options } = await post<{ challenge_id: string; options: any }>('/auth/passkey/login/start', { email });
  const pk = options.publicKey;
  const cred = (await navigator.credentials.get({
    publicKey: {
      ...pk,
      challenge: b64urlToBuf(pk.challenge),
      allowCredentials: (pk.allowCredentials ?? []).map((c: any) => ({ ...c, id: b64urlToBuf(c.id) })),
    },
  })) as PublicKeyCredential | null;
  if (!cred) throw new Error('已取消');
  const r = cred.response as AuthenticatorAssertionResponse;
  const credential = {
    id: cred.id,
    rawId: bufToB64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: bufToB64url(r.authenticatorData),
      clientDataJSON: bufToB64url(r.clientDataJSON),
      signature: bufToB64url(r.signature),
      userHandle: bufToB64url(r.userHandle),
    },
    extensions: cred.getClientExtensionResults(),
  };
  await post('/auth/passkey/login/finish', { challenge_id, credential });
}
