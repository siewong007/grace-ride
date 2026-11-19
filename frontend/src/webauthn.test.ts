import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  clearDevicePasskey,
  getDevicePasskey,
  loginWithPasskey,
  saveDevicePasskey,
  withRecentSignIn,
} from './webauthn';

const { post } = vi.hoisted(() => ({ post: vi.fn() }));
vi.mock('./api', async (orig) => ({ ...(await orig<typeof import('./api')>()), post }));

const tooOld = () => new ApiError(403, 'sign in again', 'RECENT_SIGN_IN_REQUIRED');

describe('withRecentSignIn', () => {
  beforeEach(() => post.mockReset());

  it('passes the error on without a passkey prompt when the member has no usable passkey', async () => {
    const fn = vi.fn().mockRejectedValue(tooOld());
    await expect(withRecentSignIn(fn, false)).rejects.toMatchObject({ code: 'RECENT_SIGN_IN_REQUIRED' });
    expect(fn).toHaveBeenCalledTimes(1);
    expect(post).not.toHaveBeenCalled();
  });

  it('passes other errors on untouched', async () => {
    const fn = vi.fn().mockRejectedValue(new ApiError(400, 'nope', 'OTHER'));
    await expect(withRecentSignIn(fn, true)).rejects.toMatchObject({ code: 'OTHER' });
    expect(post).not.toHaveBeenCalled();
  });

  it('asks for the passkey once and retries when the member has one', async () => {
    const fn = vi.fn().mockRejectedValueOnce(tooOld()).mockResolvedValueOnce('done');
    post
      .mockResolvedValueOnce({ challenge_id: 'c1', options: { publicKey: { challenge: 'AA', allowCredentials: [] } } })
      .mockResolvedValueOnce({ ok: true });
    const cred = {
      id: 'x',
      rawId: new Uint8Array([1]).buffer,
      type: 'public-key',
      response: {
        authenticatorData: new Uint8Array([1]).buffer,
        clientDataJSON: new Uint8Array([1]).buffer,
        signature: new Uint8Array([1]).buffer,
        userHandle: new Uint8Array([1]).buffer,
      },
      getClientExtensionResults: () => ({}),
    };
    vi.stubGlobal('navigator', { ...navigator, credentials: { get: vi.fn().mockResolvedValue(cred) } });
    await expect(withRecentSignIn(fn, true)).resolves.toBe('done');
    expect(post).toHaveBeenCalledTimes(2);
    expect(fn).toHaveBeenCalledTimes(2);
    expect(post).toHaveBeenCalledWith('/auth/passkey/reauth/start', {});
    expect(post.mock.calls[1][0]).toBe('/auth/passkey/reauth/finish');
    vi.unstubAllGlobals();
  });
});

describe('device passkey helpers', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('saves and reads device passkey info with lowercased email', () => {
    expect(getDevicePasskey()).toBeNull();
    saveDevicePasskey('Alice@example.com');
    const info = getDevicePasskey();
    expect(info).not.toBeNull();
    expect(info?.email).toBe('alice@example.com');
    expect(info?.savedAt).toBeTypeOf('number');
  });

  it('clears device passkey info', () => {
    saveDevicePasskey('alice@example.com');
    expect(getDevicePasskey()).not.toBeNull();
    clearDevicePasskey();
    expect(getDevicePasskey()).toBeNull();
  });

  it('ignores empty email strings', () => {
    saveDevicePasskey('');
    expect(getDevicePasskey()).toBeNull();
  });
});

describe('loginWithPasskey', () => {
  beforeEach(() => {
    post.mockReset();
    localStorage.clear();
  });

  it('saves device passkey on successful login', async () => {
    post
      .mockResolvedValueOnce({ challenge_id: 'c1', options: { publicKey: { challenge: 'AA', allowCredentials: [] } } })
      .mockResolvedValueOnce({ ok: true });
    const cred = {
      id: 'cred1',
      rawId: new Uint8Array([1]).buffer,
      type: 'public-key',
      response: {
        authenticatorData: new Uint8Array([1]).buffer,
        clientDataJSON: new Uint8Array([1]).buffer,
        signature: new Uint8Array([1]).buffer,
        userHandle: new Uint8Array([1]).buffer,
      },
      getClientExtensionResults: () => ({}),
    };
    vi.stubGlobal('navigator', { ...navigator, credentials: { get: vi.fn().mockResolvedValue(cred) } });

    await loginWithPasskey('Bob@example.com');
    expect(post).toHaveBeenCalledWith('/auth/passkey/login/start', { email: 'bob@example.com' });
    expect(post).toHaveBeenCalledWith('/auth/passkey/login/finish', expect.anything());
    expect(getDevicePasskey()?.email).toBe('bob@example.com');
    vi.unstubAllGlobals();
  });
});
