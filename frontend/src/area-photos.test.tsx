import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import i18n from './i18n';
import { ConfirmProvider } from './components/ui';
import { AreaPhotoStrip } from './components/AreaPhotos';
import AreaPhotos from './pages/admin/AreaPhotos';
import { fitWithin } from './cropImage';

const mocks = vi.hoisted(() => ({
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
  upload: vi.fn(),
  photos: null as unknown,
}));

vi.mock('./api', () => ({
  post: mocks.post,
  put: mocks.put,
  del: mocks.del,
  uploadFile: mocks.upload,
  useApi: (path: string | null) => ({
    data:
      path === '/areas'
        ? [
            { id: 1, name: 'UT', active: true, photo_count: 2 },
            { id: 2, name: 'UM', active: true, photo_count: 0 },
          ]
        : path?.startsWith('/areas/')
          ? mocks.photos
          : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));
vi.mock('./cropImage', async (orig) => ({
  ...(await orig<typeof import('./cropImage')>()),
  shrinkPhoto: (f: File) => Promise.resolve(f),
}));

const photo = (id: number, caption: string, own: boolean) => ({
  id,
  file_id: 100 + id,
  url: `/api/files/${100 + id}`,
  caption,
  sort_order: id,
  can_edit: own,
  can_delete: own,
  mine: own,
  created_by: { id: own ? 9 : 5, name: own ? 'Amina' : 'Head' },
});

beforeAll(async () => {
  await i18n.changeLanguage('en');
});
beforeEach(() => {
  mocks.post.mockReset().mockResolvedValue({ id: 1 });
  mocks.put.mockReset().mockResolvedValue({});
  mocks.del.mockReset().mockResolvedValue({});
  mocks.upload.mockReset().mockResolvedValue({ id: 555, mime: 'image/jpeg', name: 'a.jpg' });
  mocks.photos = {
    area: { id: 1, name: 'UT' },
    photos: [photo(1, 'Wait at the guardhouse', false), photo(2, '', true)],
    max: 10,
    can_add: true,
    can_reorder: false,
  };
});

describe('pickup area photos', () => {
  it('shows a passenger the area photos and enlarges one with next / previous', () => {
    render(<AreaPhotoStrip areaId={1} areaName="UT" />);
    expect(screen.getByText('Where to wait in UT')).toBeInTheDocument();
    expect(screen.getByText('Wait at the guardhouse')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Photo 1 of 2/ }));
    const viewer = screen.getByRole('dialog', { name: 'UT' });
    expect(within(viewer).getByRole('img')).toHaveAttribute('src', '/api/files/101');
    expect(within(viewer).getByText('UT · 1 / 2')).toBeInTheDocument();
    expect(within(viewer).getByRole('button', { name: 'Previous photo' })).toBeDisabled();
    fireEvent.click(within(viewer).getByRole('button', { name: 'Next photo' }));
    expect(within(viewer).getByRole('img')).toHaveAttribute('src', '/api/files/102');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows nothing when an area has no photos', () => {
    mocks.photos = { area: { id: 2, name: 'UM' }, photos: [], max: 10, can_add: false, can_reorder: false };
    const { container } = render(<AreaPhotoStrip areaId={2} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('lets a team member add photos and change or remove only their own', async () => {
    render(
      <ConfirmProvider>
        <AreaPhotos />
      </ConfirmProvider>,
    );
    expect(screen.getByText('2 of 10 photos')).toBeInTheDocument();
    // No reordering for a team member; edit and remove only on their own photo.
    expect(screen.queryByRole('button', { name: 'Move earlier' })).not.toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: 'Remove photo' })).toHaveLength(1);
    expect(screen.getAllByRole('button', { name: 'Edit caption' })).toHaveLength(1);
    expect(screen.getByText('Added by you')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/Caption/), { target: { value: ' Gate B ' } });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const files = [new File(['a'], 'a.jpg', { type: 'image/jpeg' }), new File(['b'], 'b.png', { type: 'image/png' })];
    fireEvent.change(input, { target: { files } });
    await waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(2));
    expect(mocks.upload).toHaveBeenCalledTimes(2);
    expect(mocks.post).toHaveBeenCalledWith('/areas/1/photos', { file_id: 555, caption: 'Gate B' });

    fireEvent.click(screen.getByRole('button', { name: 'Remove photo' }));
    fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Remove photo' }));
    await waitFor(() => expect(mocks.del).toHaveBeenCalledWith('/areas/1/photos/2'));
  });

  it('lets the traffic lead reorder photos', async () => {
    mocks.photos = { ...(mocks.photos as object), can_reorder: true };
    render(
      <ConfirmProvider>
        <AreaPhotos />
      </ConfirmProvider>,
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Move later' })[0]);
    await waitFor(() => expect(mocks.put).toHaveBeenCalledWith('/areas/1/photos/order', { photo_ids: [2, 1] }));
  });

  it('refuses more photos than the area has room for', async () => {
    mocks.photos = { ...(mocks.photos as object), max: 3 };
    render(
      <ConfirmProvider>
        <AreaPhotos />
      </ConfirmProvider>,
    );
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const files = [new File(['a'], 'a.jpg', { type: 'image/jpeg' }), new File(['b'], 'b.jpg', { type: 'image/jpeg' })];
    fireEvent.change(input, { target: { files } });
    expect(await screen.findByText('Only 1 more photo fits in this area.')).toBeInTheDocument();
    expect(mocks.upload).not.toHaveBeenCalled();
  });

  it('shrinks photos to fit without enlarging small ones', () => {
    expect(fitWithin(4000, 3000, 1600)).toEqual({ w: 1600, h: 1200 });
    expect(fitWithin(900, 1600 * 2, 1600)).toEqual({ w: 450, h: 1600 });
    expect(fitWithin(800, 600, 1600)).toEqual({ w: 800, h: 600 });
  });
});
