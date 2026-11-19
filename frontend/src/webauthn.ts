// Minimal WebAuthn helpers translating between webauthn-rs JSON (base64url) and the browser API.
import { post, type ApiError } from './api';
import i18n from './i18n';

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

/** Credential descriptor as the server sends it: id is base64url text. */
type ServerCredential = { id: string } & Record<string, unknown>;
/** WebAuthn options from the server, with binary fields as base64url strings. */
interface ServerPasskeyOptions {
  publicKey: Record<string, unknown> & {
    challenge: string;
    user?: { id: string } & Record<string, unknown>;
    excludeCredentials?: ServerCredential[];
    allowCredentials?: ServerCredential[];
  };
}

export const passkeySupported = () => typeof window !== 'undefined' && !!window.PublicKeyCredential;

export async function registerPasskey(label?: string, email?: string) {
  const { challenge_id, options } = await post<{ challenge_id: string; options: ServerPasskeyOptions }>('/auth/passkey/register/start');
  const pk = options.publicKey;
  const cred = (await navigator.credentials.create({
    publicKey: {
      ...pk,
      challenge: b64urlToBuf(pk.challenge),
      user: { ...pk.user, id: b64urlToBuf(pk.user!.id) },
      excludeCredentials: (pk.excludeCredentials ?? []).map((c) => ({ ...c, id: b64urlToBuf(c.id) })),
    } as unknown as PublicKeyCredentialCreationOptions,
  })) as PublicKeyCredential | null;
  if (!cred) throw new Error(i18n.t('auth:cancelled'));
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
  if (email) {
    saveDevicePasskey(email);
  }
}

/** Asks the browser for a passkey assertion and posts it to `finishPath`. */
async function assertPasskey(startPath: string, startBody: unknown, finishPath: string) {
  const { challenge_id, options } = await post<{ challenge_id: string; options: ServerPasskeyOptions }>(startPath, startBody);
  const pk = options.publicKey;
  const cred = (await navigator.credentials.get({
    publicKey: {
      ...pk,
      challenge: b64urlToBuf(pk.challenge),
      allowCredentials: (pk.allowCredentials ?? []).map((c) => ({ ...c, id: b64urlToBuf(c.id) })),
    } as unknown as PublicKeyCredentialRequestOptions,
  })) as PublicKeyCredential | null;
  if (!cred) throw new Error(i18n.t('auth:cancelled'));
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
  await post(finishPath, { challenge_id, credential });
}

export const loginWithPasskey = async (email: string) => {
  const normEmail = email.trim().toLowerCase();
  await assertPasskey('/auth/passkey/login/start', { email: normEmail }, '/auth/passkey/login/finish');
  saveDevicePasskey(normEmail);
};

/** Confirms the signed-in member with their passkey, without signing out. */
export const reauthWithPasskey = () =>
  assertPasskey('/auth/passkey/reauth/start', {}, '/auth/passkey/reauth/finish');

/**
 * Runs a security change. If the server says the sign-in is too old and the member can
 * use a passkey here, asks for it once and retries, so nobody has to sign out and in
 * again. Without a usable passkey the server's error is passed on as is: it tells the
 * member to sign out and back in instead.
 */
export async function withRecentSignIn<T>(fn: () => Promise<T>, canPasskey: boolean): Promise<T> {
  try {
    return await fn();
  } catch (e) {
    if (!canPasskey || (e as ApiError).code !== 'RECENT_SIGN_IN_REQUIRED') throw e;
    await reauthWithPasskey();
    return fn();
  }
}

const DEVICE_PASSKEY_KEY = 'gr_device_passkey';

export interface DevicePasskeyInfo {
  email: string;
  savedAt: number;
}

export function getDevicePasskey(): DevicePasskeyInfo | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const raw = localStorage.getItem(DEVICE_PASSKEY_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed.email === 'string' && parsed.email.trim()) {
      return {
        email: parsed.email.trim().toLowerCase(),
        savedAt: typeof parsed.savedAt === 'number' ? parsed.savedAt : Date.now(),
      };
    }
  } catch {
    /* ignore parsing errors */
  }
  return null;
}

export function saveDevicePasskey(email: string): void {
  if (typeof localStorage === 'undefined' || !email || !email.trim()) return;
  try {
    const info: DevicePasskeyInfo = {
      email: email.trim().toLowerCase(),
      savedAt: Date.now(),
    };
    localStorage.setItem(DEVICE_PASSKEY_KEY, JSON.stringify(info));
  } catch {
    /* ignore storage errors */
  }
}

export function clearDevicePasskey(): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.removeItem(DEVICE_PASSKEY_KEY);
  } catch {
    /* ignore storage errors */
  }
}

export const platformAuthenticatorAvailable = async (): Promise<boolean> => {
  if (typeof window === 'undefined' || !window.PublicKeyCredential || !PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable) {
    return false;
  }
  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
};
