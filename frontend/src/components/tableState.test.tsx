import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { useTableParams } from './tableState';

function Harness() {
  const location = useLocation();
  const table = useTableParams({
    defaultPageSize: 25,
    defaultSort: { key: 'name', dir: 'asc' },
    filterKeys: ['q', 'status'],
  });
  return (
    <div>
      <output data-testid="search">{location.search}</output>
      <output data-testid="page">{table.page}</output>
      <output data-testid="sort">
        {table.sort ? `${table.sort.key}:${table.sort.dir}` : 'none'}
      </output>
      <output data-testid="query">{table.queryString}</output>
      <output data-testid="filters">{JSON.stringify(table.filters)}</output>
      <button onClick={() => table.setPage(3)}>page3</button>
      <button onClick={() => table.setPageSize(50)}>size50</button>
      <button onClick={() => table.setFilter('q', 'ana')}>search</button>
      <button onClick={() => table.setSort({ key: 'name', dir: 'desc' })}>sort</button>
      <button onClick={table.clearFilters}>clear</button>
    </div>
  );
}

describe('useTableParams', () => {
  it('serializes filters/sort, resets the page, and preserves unrelated params', () => {
    render(
      <MemoryRouter
        initialEntries={['/members?scope=keep&q=bo&page=4&sort=name&dir=desc']}
      >
        <Harness />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('page')).toHaveTextContent('4');
    expect(screen.getByTestId('sort')).toHaveTextContent('name:desc');
    expect(screen.getByTestId('query')).toHaveTextContent(
      'page=4&sort=name&dir=desc&q=bo',
    );
    expect(screen.getByTestId('filters')).toHaveTextContent('{"q":"bo","status":""}');

    fireEvent.click(screen.getByRole('button', { name: 'search' }));
    expect(screen.getByTestId('search')).toHaveTextContent(
      '?scope=keep&q=ana&sort=name&dir=desc',
    );
    expect(screen.getByTestId('page')).toHaveTextContent('1');
    expect(screen.getByTestId('query')).toHaveTextContent('sort=name&dir=desc&q=ana');

    fireEvent.click(screen.getByRole('button', { name: 'clear' }));
    expect(screen.getByTestId('search')).toHaveTextContent('?scope=keep');
    expect(screen.getByTestId('sort')).toHaveTextContent('name:asc');
    expect(screen.getByTestId('query').textContent).toBe('');
  });

  it('omits a sort that equals the default', () => {
    render(
      <MemoryRouter initialEntries={['/members?page=2&sort=name&dir=asc']}>
        <Harness />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('query')).toHaveTextContent('page=2');
  });

  it('changes page size and resets the page', () => {
    render(
      <MemoryRouter initialEntries={['/members?page=4']}>
        <Harness />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'size50' }));
    expect(screen.getByTestId('search')).toHaveTextContent('?limit=50');
    expect(screen.getByTestId('page')).toHaveTextContent('1');
    expect(screen.getByTestId('query')).toHaveTextContent('limit=50');
  });
});
