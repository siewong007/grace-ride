import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { ConfirmProvider, ToastProvider } from '../../components/ui';
import { buildFaresXlsx } from '../../fareSheet';
import { buildXlsxWorkbook } from '../../xlsxCodec';
import AreaFares, { type AreaFare, type FareArea } from './AreaFares';

const mocks = vi.hoisted(() => ({
  put: vi.fn(),
  reloadFares: vi.fn(),
  reloadAreas: vi.fn(),
  can: (() => true) as (page: string, act?: string) => boolean,
  areas: [] as FareArea[],
  fares: [] as AreaFare[],
}));

vi.mock('../../api', () => ({
  put: mocks.put,
  post: vi.fn(),
  del: vi.fn(),
  useApi: (path: string | null) => {
    if (path === '/areas') {
      return { data: mocks.areas, loading: false, error: null, reload: mocks.reloadAreas };
    }
    if (path === '/admin/settings/fares') {
      return {
        data: { fares: mocks.fares },
        loading: false,
        error: null,
        reload: mocks.reloadFares,
      };
    }
    return { data: null, loading: false, error: null, reload: vi.fn() };
  },
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({ can: (page: string, act?: string) => mocks.can(page, act) }),
}));

function makeTestFile(content: Uint8Array | string, name: string, type: string): File {
  const bytes = typeof content === 'string' ? new TextEncoder().encode(content) : content;
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  const file = new File([copy], name, { type });
  if (typeof file.arrayBuffer !== 'function') {
    Object.defineProperty(file, 'arrayBuffer', {
      value: async () => copy.buffer.slice(0),
    });
  }
  return file;
}

function renderAreaFares() {
  return render(
    <MemoryRouter>
      <ToastProvider>
        <ConfirmProvider>
          <AreaFares />
        </ConfirmProvider>
      </ToastProvider>
    </MemoryRouter>,
  );
}

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  mocks.areas = [
    { id: 1, name: 'Cheras', active: true },
    { id: 2, name: 'PJ', active: true },
    { id: 3, name: 'KK', active: true },
  ];
  mocks.fares = [
    {
      id: 7,
      area_a: { id: 1, name: 'Cheras' },
      area_b: { id: 2, name: 'PJ' },
      amount_cents: 1500,
      updated_at: '',
    },
  ];
  mocks.put.mockReset().mockResolvedValue({
    created: 1,
    updated: 1,
    unchanged: 0,
    total: 2,
    areas_created: [],
  });
  mocks.reloadFares.mockReset();
  mocks.reloadAreas.mockReset();
  mocks.can = () => true;
});

describe('AreaFares bulk import & export', () => {
  it('shows export buttons to viewers and hides import button when read-only', () => {
    mocks.can = (page, act) => page === 'admin_fares' && act !== 'update';
    renderAreaFares();

    expect(screen.getByRole('button', { name: 'Download Excel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Download CSV' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Import Excel / CSV' })).not.toBeInTheDocument();
  });

  it('parses a CSV file, previews diffs, and applies bulk fares', async () => {
    renderAreaFares();

    fireEvent.click(screen.getByRole('button', { name: 'Import Excel / CSV' }));
    const input = screen.getByLabelText(/Choose a \.xlsx or \.csv file/i) as HTMLInputElement;
    const csv = [
      'From,To,Fare (RM)',
      'Cheras,PJ,18.00',
      'Cheras,KK,12.50',
      'PJ,KK,',
    ].join('\n');

    fireEvent.change(input, {
      target: { files: [makeTestFile(csv, 'fares.csv', 'text/csv')] },
    });

    expect(await screen.findByText('1 new')).toBeInTheDocument();
    expect(screen.getByText('1 updated')).toBeInTheDocument();
    expect(screen.getByText('0 unchanged')).toBeInTheDocument();

    const applyBtn = screen.getByRole('button', { name: 'Apply 2 changes' });
    expect(applyBtn).toBeEnabled();
    fireEvent.click(applyBtn);

    await waitFor(() => {
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/fares/bulk', {
        fares: [
          { area_a: 'Cheras', area_b: 'PJ', amount_cents: 1800 },
          { area_a: 'Cheras', area_b: 'KK', amount_cents: 1250 },
        ],
        create_missing_areas: false,
      });
    });
    expect(mocks.reloadFares).toHaveBeenCalledTimes(1);
    expect(mocks.reloadAreas).not.toHaveBeenCalled();
  });

  it('imports a real .xlsx matrix built from buildFaresXlsx and handles unknown areas by permission', async () => {
    mocks.put.mockResolvedValue({
      created: 1,
      updated: 0,
      unchanged: 0,
      total: 1,
      areas_created: ['Subang'],
    });
    renderAreaFares();

    fireEvent.click(screen.getByRole('button', { name: 'Import Excel / CSV' }));
    const input = screen.getByLabelText(/Choose a \.xlsx or \.csv file/i) as HTMLInputElement;

    // Generate a matrix workbook including an unknown area 'Subang'.
    const extendedAreas: FareArea[] = [
      ...mocks.areas,
      { id: 99, name: 'Subang', active: true },
    ];
    const xlsxBytes = buildFaresXlsx(
      extendedAreas,
      [
        ...mocks.fares,
        {
          id: 9,
          area_a: { id: 1, name: 'Cheras' },
          area_b: { id: 99, name: 'Subang' },
          amount_cents: 2400,
          updated_at: '',
        },
      ],
      { sheetName: 'Area Fares', originLabel: 'From \\ To' },
    );

    fireEvent.change(input, {
      target: {
        files: [
          makeTestFile(
            xlsxBytes,
            'fares.xlsx',
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
          ),
        ],
      },
    });

    expect(await screen.findByText('Unknown areas found: Subang')).toBeInTheDocument();
    expect(screen.getByText('1 skipped')).toBeInTheDocument();
    const applyBefore = screen.getByRole('button', { name: 'Apply 0 changes' });
    expect(applyBefore).toBeDisabled();

    // Opt in to creating missing areas.
    const checkbox = screen.getByRole('checkbox', {
      name: /Also create 1 new areas and import their fares/i,
    });
    fireEvent.click(checkbox);
    const applyAfter = screen.getByRole('button', { name: 'Apply 1 changes' });
    expect(applyAfter).toBeEnabled();
    fireEvent.click(applyAfter);

    await waitFor(() => {
      expect(mocks.put).toHaveBeenCalledWith('/admin/settings/fares/bulk', {
        fares: [{ area_a: 'Cheras', area_b: 'Subang', amount_cents: 2400 }],
        create_missing_areas: true,
      });
    });
    expect(mocks.reloadFares).toHaveBeenCalled();
    expect(mocks.reloadAreas).toHaveBeenCalled();
  });

  it('hides missing-area creation option without admin_areas.create and blocks apply on conflicts/invalid cells', async () => {
    mocks.can = (page, act) => page === 'admin_fares' && (act === 'view' || act === 'update');
    renderAreaFares();

    fireEvent.click(screen.getByRole('button', { name: 'Import Excel / CSV' }));
    const input = screen.getByLabelText(/Choose a \.xlsx or \.csv file/i) as HTMLInputElement;

    const xlsx = buildXlsxWorkbook('Sheet1', [
      [
        { value: 'From \\ To' },
        { value: 'Cheras' },
        { value: 'PJ' },
        { value: 'Subang' },
      ],
      [{ value: 'Cheras' }, { value: 5 }, { value: 15 }, { value: 20 }],
      [{ value: 'PJ' }, { value: 19 }, { value: 'oops' }, { value: '' }],
    ]);

    fireEvent.change(input, {
      target: {
        files: [
          makeTestFile(
            xlsx,
            'bad.xlsx',
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
          ),
        ],
      },
    });

    expect(await screen.findByText(/Fix these issues in the file before applying/i)).toBeInTheDocument();
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
    expect(screen.getByText(/Unknown areas found: Subang/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Apply/i })).toBeDisabled();
  });
});
