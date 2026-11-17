import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';

const { post } = vi.hoisted(() => ({ post: vi.fn() }));
vi.mock('./api', async (orig) => ({ ...(await orig<typeof import('./api')>()), post }));

import { withRecentSignIn } from './webauthn';

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
