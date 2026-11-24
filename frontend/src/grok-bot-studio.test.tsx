import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import './i18n';
import i18n from './i18n';
import { put } from './api';
import AvatarEditor from './components/AvatarEditor';
import { Avatar, ToastProvider } from './components/ui';
import {
  buildStandaloneGrokBotSvg,
  encodeGrokBotIcon,
  GROKBOT_COLOR_PALETTES,
  GROKBOT_COLORS,
  GROKBOT_FACES,
  GROKBOT_MOVES,
  GROKBOT_SHAPES,
  parseGrokBotIcon,
  SHAPE_COORDS,
} from './grokBotStudio';
import GrokBotStudioPage from './pages/GrokBotStudioPage';

const mockRefresh = vi.fn().mockResolvedValue(undefined);

vi.mock('./api', () => ({
  put: vi.fn().mockResolvedValue({ ok: true }),
  uploadFile: vi.fn(),
}));

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: {
      id: 7,
      name: 'Grace Member',
      email: 'member@example.org',
      avatar_icon: null,
      avatar_color: null,
      photo_file_id: null,
    },
    loading: false,
    refresh: mockRefresh,
    can: () => true,
  }),
}));

function LocationProbe() {
  const loc = useLocation();
  return <div data-testid="loc-search">{loc.search}</div>;
}

function renderStudio(initialEntry = '/studio?shape=cloud&color=brown&face=excited') {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <ToastProvider>
        <LocationProbe />
        <Routes>
          <Route path="/studio" element={<GrokBotStudioPage />} />
        </Routes>
      </ToastProvider>
    </MemoryRouter>,
  );
}

describe('Grok Bot Studio', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage('en');
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
        write: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  it('validates all 8 shapes, 16 faces, 12 colours, 15 moves, and 64-segment geometry', () => {
    expect(GROKBOT_SHAPES).toHaveLength(8);
    expect(GROKBOT_FACES).toHaveLength(16);
    expect(GROKBOT_COLORS).toHaveLength(12);
    expect(GROKBOT_MOVES).toHaveLength(15);

    for (const sh of GROKBOT_SHAPES) {
      expect(SHAPE_COORDS[sh]).toHaveLength(386);
    }

    expect(parseGrokBotIcon('grokbot')).toEqual({
      shape: 'circle',
      face: 'neutral',
      color: 'blue',
    });
    expect(parseGrokBotIcon('grokbot:cloud:excited:brown')).toEqual({
      shape: 'cloud',
      face: 'excited',
      color: 'brown',
    });
    expect(parseGrokBotIcon('grokbot:bad:excited:brown')).toBeNull();
    expect(parseGrokBotIcon('grok:circle:coral')).toBeNull();
    expect(encodeGrokBotIcon({ shape: 'droplet', face: 'curious', color: 'turquoise' })).toBe(
      'grokbot:droplet:curious:turquoise',
    );

    const svg = buildStandaloneGrokBotSvg({ shape: 'cloud', face: 'excited', color: 'brown' }, 512);
    expect(svg).toContain('width="512" height="512"');
    expect(svg).toContain('<mask id="gb-export-mask"');
    expect(svg).toContain('<radialGradient id="gb-export-fill"');
    expect(svg).toContain(GROKBOT_COLOR_PALETTES.brown.hex);
  });

  it('renders /studio from URL params, syncs shape/face/colour & arrow keys, and toggles Move animations', async () => {
    renderStudio('/studio?shape=cloud&color=brown&face=excited');

    expect(screen.getByTestId('studio-status-readout')).toHaveTextContent('BROWN CLOUD · EXCITED');
    expect(screen.getByText('It looks at your cursor.')).toBeInTheDocument();

    const shapeGroup = screen.getByRole('radiogroup', { name: 'Shape' });
    const faceGroup = screen.getByRole('radiogroup', { name: 'Face' });
    const colorGroup = screen.getByRole('radiogroup', { name: 'Colour' });

    expect(within(shapeGroup).getAllByRole('radio')).toHaveLength(8);
    expect(within(faceGroup).getAllByRole('radio')).toHaveLength(16);
    expect(within(colorGroup).getAllByRole('radio')).toHaveLength(12);

    // Click Hexagon shape + Turquoise colour + Happy face
    fireEvent.click(within(shapeGroup).getByRole('radio', { name: /Hexagon/i }));
    fireEvent.click(within(colorGroup).getByRole('radio', { name: 'Turquoise' }));
    fireEvent.click(within(faceGroup).getByRole('radio', { name: /Happy/i }));

    expect(screen.getByTestId('studio-status-readout')).toHaveTextContent('TURQUOISE HEXAGON · HAPPY');
    expect(screen.getByTestId('loc-search').textContent).toContain('shape=hexagon');
    expect(screen.getByTestId('loc-search').textContent).toContain('color=turquoise');
    expect(screen.getByTestId('loc-search').textContent).toContain('face=happy');

    // Keyboard arrow navigation on Shape radiogroup (Hexagon -> Cloud)
    const hexRadio = within(shapeGroup).getByRole('radio', { name: /Hexagon/i });
    fireEvent.keyDown(hexRadio, { key: 'ArrowRight' });
    expect(screen.getByTestId('studio-status-readout')).toHaveTextContent('TURQUOISE CLOUD · HAPPY');

    // Surprise me picks a different valid combination
    const prevSearch = screen.getByTestId('loc-search').textContent;
    fireEvent.click(screen.getByRole('button', { name: /Surprise me/i }));
    expect(screen.getByTestId('loc-search').textContent).not.toBe(prevSearch);

    // Switch to Move tab and toggle animations
    fireEvent.click(screen.getByRole('tab', { name: 'Move' }));
    const animGroup = screen.getByRole('group', { name: 'Animations' });
    const moveButtons = within(animGroup).getAllByRole('button');
    expect(moveButtons).toHaveLength(15);

    const orbitBtn = within(animGroup).getByRole('button', { name: /Orbit/i });
    expect(orbitBtn).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(orbitBtn);
    expect(orbitBtn).toHaveAttribute('aria-pressed', 'true');
    // Clicking again toggles back to Idle
    fireEvent.click(orbitBtn);
    expect(orbitBtn).toHaveAttribute('aria-pressed', 'false');
    expect(within(animGroup).getByRole('button', { name: /Idle/i })).toHaveAttribute('aria-pressed', 'true');
  });

  it('saves avatar via PUT /me and copies SVG and Studio link from Copy menu', async () => {
    renderStudio('/studio?shape=squircle&color=purple&face=proud');

    fireEvent.click(screen.getByRole('button', { name: /Save as my avatar/i }));
    await waitFor(() =>
      expect(put).toHaveBeenCalledWith('/me', {
        clear_photo: true,
        avatar_icon: 'grokbot:squircle:proud:purple',
        avatar_color: '',
      }),
    );
    expect(mockRefresh).toHaveBeenCalled();

    // Open Copy dropdown and test Copy SVG + Copy link
    fireEvent.click(screen.getByRole('button', { name: /Copy/i }));
    expect(screen.getByRole('menuitem', { name: 'Copy image' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('menuitem', { name: 'Copy SVG' }));
    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        expect.stringContaining('<svg xmlns="http://www.w3.org/2000/svg"'),
      ),
    );

    fireEvent.click(screen.getByRole('button', { name: /Copy/i }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Copy link' }));
    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        expect.stringContaining('/studio?shape=squircle&color=purple&face=proud'),
      ),
    );
  });

  it('renders GrokBotSvg in global <Avatar> and integrates Grok Studio tab in AvatarEditor', async () => {
    const { container } = render(<Avatar name="Grace" icon="grokbot:droplet:curious:ink" size="lg" />);
    const svg = container.querySelector('svg.grokbot-svg');
    expect(svg).not.toBeNull();
    expect(container.querySelector('.avatar.avatar-grokbot')).not.toBeNull();

    const onSaved = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <ToastProvider>
          <AvatarEditor
            name="Grace"
            current={{ avatar_icon: 'grokbot:droplet:curious:ink' }}
            onSaved={onSaved}
            onClose={onClose}
          />
        </ToastProvider>
      </MemoryRouter>,
    );

    expect(screen.getByRole('tab', { name: 'Grok Studio' })).toHaveAttribute('aria-selected', 'true');
    const shapeGroup = screen.getByRole('radiogroup', { name: 'Shape' });
    fireEvent.click(within(shapeGroup).getByRole('radio', { name: /Capsule/i }));
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() =>
      expect(put).toHaveBeenCalledWith('/me', {
        clear_photo: true,
        avatar_icon: 'grokbot:capsule:curious:ink',
        avatar_color: '',
      }),
    );
    expect(onSaved).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });
});
