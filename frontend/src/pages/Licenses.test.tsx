import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import Licenses from './Licenses';

const packages = [
  { name: 'react', version: '18.3.1', license: 'MIT', text: 'MIT License\nPermission is hereby granted.' },
  { name: 'leaflet', version: '1.9.4', license: 'BSD-2-Clause', text: 'BSD 2-Clause License' },
];

const renderPage = () =>
  render(
    <MemoryRouter>
      <Licenses />
    </MemoryRouter>,
  );

afterEach(() => vi.unstubAllGlobals());

describe('Licenses page', () => {
  it('lists every package with its licence and the full text', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => packages }));
    renderPage();

    expect(await screen.findByText('react')).toBeInTheDocument();
    expect(screen.getByText('leaflet')).toBeInTheDocument();
    expect(screen.getByText('BSD-2-Clause')).toBeInTheDocument();
    expect(screen.getByText(/Permission is hereby granted/)).toBeInTheDocument();
    expect(screen.getByText('2 packages')).toBeInTheDocument();
  });

  it('filters the list by package name', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, json: async () => packages }));
    renderPage();

    fireEvent.change(await screen.findByRole('searchbox'), { target: { value: 'leaf' } });

    await waitFor(() => expect(screen.queryByText('react')).not.toBeInTheDocument());
    expect(screen.getByText('leaflet')).toBeInTheDocument();
  });

  it('says so when the notices cannot be loaded', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));
    renderPage();

    expect(await screen.findByText(/could not be loaded/)).toBeInTheDocument();
  });
});
