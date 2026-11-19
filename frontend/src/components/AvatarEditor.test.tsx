import { useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { put, uploadFile } from '../api';
import { cropToFile, loadImage } from '../cropImage';
import AvatarEditor from './AvatarEditor';

vi.mock('../api', () => ({
  put: vi.fn().mockResolvedValue({ ok: true }),
  uploadFile: vi.fn().mockResolvedValue({ id: 77, mime: 'image/jpeg', name: 'avatar.jpg' }),
}));

// jsdom cannot decode images or draw to a canvas; the geometry helpers stay real.
vi.mock('../cropImage', async (orig) => ({
  ...(await orig<typeof import('../cropImage')>()),
  loadImage: vi.fn(),
  cropToFile: vi.fn(),
}));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }),
}));

vi.mock('./ui', async () => {
  const { Avatar, ErrorBox } = await import('./ui/Feedback');
  const { Tabs } = await import('./ui/Tabs');
  return {
    Avatar,
    ErrorBox,
    Tabs,
    Button: ({ children, loading: _l, variant: _v, size: _s, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement> & { loading?: boolean; variant?: string; size?: string }) => (
      <button type="button" {...rest}>{children}</button>
    ),
    Modal: ({ title, children, footer }: { title: string; children: React.ReactNode; footer?: React.ReactNode }) => (
      <div role="dialog" aria-label={title}>{children}{footer}</div>
    ),
    useAction: () => {
      const [error, setError] = useState<string | null>(null);
      return {
        busy: false,
        error,
        run: async (fn: () => Promise<unknown>) => {
          setError(null);
          try {
            await fn();
          } catch (e) {
            setError((e as Error).message);
          }
        },
      };
    },
  };
});

const setup = (current = {}) => {
  const onSaved = vi.fn().mockResolvedValue(undefined);
  const onClose = vi.fn();
  render(<AvatarEditor name="Grace" current={current} onSaved={onSaved} onClose={onClose} />);
  return { onSaved, onClose };
};

beforeEach(() => vi.clearAllMocks());

describe('AvatarEditor', () => {
  it('saves an initial on a chosen colour and clears any photo', async () => {
    const { onSaved, onClose } = setup({ photo_file_id: 5 });
    fireEvent.click(screen.getByRole('tab', { name: 'avatarInitials' }));
    fireEvent.click(screen.getByRole('radio', { name: 'avatarColor_teal' }));
    fireEvent.click(screen.getByRole('button', { name: 'avatarSave' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(put).toHaveBeenCalledWith('/me', { clear_photo: true, avatar_icon: '', avatar_color: 'teal' });
    expect(onSaved).toHaveBeenCalled();
  });

  it('requires an icon before saving icon mode, then saves it', async () => {
    const { onClose } = setup();
    fireEvent.click(screen.getByRole('tab', { name: 'avatarIcon' }));
    fireEvent.click(screen.getByRole('button', { name: 'avatarSave' }));
    expect(await screen.findByText('avatarPickIcon')).toBeInTheDocument();
    expect(put).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('radio', { name: 'avatarIcon_car' }));
    fireEvent.click(screen.getByRole('button', { name: 'avatarSave' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(put).toHaveBeenCalledWith('/me', { clear_photo: true, avatar_icon: 'car', avatar_color: '' });
  });

  it('opens on the current style and offers Remove only when customised', () => {
    setup({ avatar_icon: 'star', avatar_color: 'pink' });
    expect(screen.getByRole('tab', { name: 'avatarIcon' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('radio', { name: 'avatarIcon_star' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'avatarColor_pink' })).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'avatarRemove' }));
    expect(put).toHaveBeenCalledWith('/me', { clear_photo: true, avatar_icon: '', avatar_color: '' });
  });

  it('has no Remove button for a default avatar', () => {
    setup();
    expect(screen.queryByRole('button', { name: 'avatarRemove' })).not.toBeInTheDocument();
  });

  it('crops, uploads, and saves a picked photo', async () => {
    const img = Object.assign(document.createElement('img'), {});
    Object.defineProperty(img, 'naturalWidth', { value: 800 });
    Object.defineProperty(img, 'naturalHeight', { value: 600 });
    vi.mocked(loadImage).mockResolvedValue({ img, url: 'blob:preview' });
    const cropped = new File(['x'], 'avatar.jpg', { type: 'image/jpeg' });
    vi.mocked(cropToFile).mockResolvedValue(cropped);
    const { onClose } = setup();
    fireEvent.click(screen.getByRole('tab', { name: 'avatarPhoto' }));
    fireEvent.change(screen.getByLabelText('avatarChoosePhoto'), {
      target: { files: [new File(['raw'], 'me.png', { type: 'image/png' })] },
    });
    expect(await screen.findByLabelText('avatarZoom')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'avatarSave' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(cropToFile).toHaveBeenCalledWith(img, { view: 240, zoom: 1, x: 0, y: 0 });
    expect(uploadFile).toHaveBeenCalledWith(cropped);
    expect(put).toHaveBeenCalledWith('/me', { photo_file_id: 77, avatar_color: '' });
  });

  it('explains when the browser cannot read the picked image', async () => {
    vi.mocked(loadImage).mockRejectedValue(new Error('unreadable'));
    setup();
    fireEvent.click(screen.getByRole('tab', { name: 'avatarPhoto' }));
    fireEvent.change(screen.getByLabelText('avatarChoosePhoto'), {
      target: { files: [new File(['raw'], 'me.heic', { type: 'image/heic' })] },
    });
    expect(await screen.findByText('avatarUnreadable')).toBeInTheDocument();
  });
});
