# Universal Table Sort & Filter — Design

Date: 2026-11-14 · Status: approved by user in session · Scope: reusable frontend table layer + allow-listed backend sorting/filtering

## Problem

`UniversalTable` (`frontend/src/components/ui.tsx`) is purely presentational. It renders columns, rows, responsive stacked cards, section rows, and row clicks, but it has no sorting or filtering. Every admin page reimplements URL state, search, page resets, and "clear filters" differently, and none of the paginated tables can sort the full dataset because the backend uses fixed `ORDER BY` clauses.

Client-side sorting would only reorder the current page. Correct sorting on the paginated admin tables needs a server contract plus a reusable table control layer.

## Decision (user-confirmed choices)

1. **Server-side sorting** for Members, Rides, and Reimbursements, using allow-listed `sort`/`dir` query parameters.
2. **Built-in filter toolbar** rendered by `UniversalTable` from declarative filter definitions.
3. **Audit keeps newest-first only** — its `before=id` keyset pagination and existing filters stay intact; no column sorting.
4. Current page layouts and existing filters are preserved except where they move into the reusable toolbar.

## Component API — `frontend/src/components/ui.tsx`

```ts
export type SortDirection = 'asc' | 'desc';

export interface TableSort {
  /** Server sort field or local column key. */
  key: string;
  dir: SortDirection;
}

export interface TableColumn<T> {
  key: string;
  header: ReactNode;
  render: (item: T, index: number) => ReactNode;
  dataLabel?: string;
  headerClassName?: string;
  cellClassName?: string;

  /** Enables the header sort control. */
  sortable?: boolean;
  /** Field used in sort state / server query; defaults to `key`. */
  sortKey?: string;
  /** Required for local sorting when the table is uncontrolled. */
  sortValue?: (item: T) => string | number | null | undefined;
  /** Accessible/mobile-sort label when `header` is not a string. */
  sortLabel?: string;
}

export type TableFilter =
  | { type: 'text'; key: string; label: string; placeholder?: string }
  | {
      type: 'select';
      key: string;
      label: string;
      allLabel?: string;
      options: { value: string; label: string }[];
    }
  | { type: 'date'; key: string; label: string }
  | { type: 'checkbox'; key: string; label: string };

export interface UniversalTableProps<T> {
  // existing props...

  /** Controlled server-side sort. When set, the component does not sort `items`. */
  sort?: TableSort | null;
  /** Initial local sort when uncontrolled. */
  defaultSort?: TableSort | null;
  onSortChange?: (sort: TableSort) => void;

  /** Declarative filter toolbar. */
  filters?: TableFilter[];
  filterValues?: Record<string, string>;
  onFilterChange?: (key: string, value: string) => void;
  onClearFilters?: () => void;
  clearFiltersLabel?: string;
  toolbarActions?: ReactNode;
}
```

### Behavior

- **Controlled mode**: `sort` is supplied. Header clicks call `onSortChange`; the page updates URL/API state. The table never reorders rows itself.
- **Local mode**: `sort` is not supplied. The table sorts a copy of `items` using each column's `sortValue`. `defaultSort` applies initially; otherwise the original order is preserved.
- **Sort cycle**: click cycles `asc → desc → asc`. `aria-sort` is set on sortable headers.
- **Mobile sorting**: headers are hidden below `767px`, so the toolbar includes a compact sort `<select>` generated from sortable columns.
- **Filters**: text, select, date, and checkbox controls render in a toolbar above the table. A "Clear filters" button appears only when at least one filter is active. `toolbarActions` holds export buttons and similar page actions.
- **Empty state**: pages render `UniversalTable` even when `items` is empty, passing `emptyContent`, so the toolbar remains available to clear filters.
- **Click safety**: row clicks ignore events originating inside `a`, `button`, `input`, `select`, `textarea`, `[role="button"]`, or `[data-row-click-ignore]`.
- **Mobile labels**: explicit `dataLabel` is honored; JSX headers must not emit an empty `data-label`.

### Local sort comparison

- `null`/`undefined` values sort last in both directions.
- Numbers compare numerically; everything else compares with `localeCompare`.
- Sorting is stable; equal values retain their original order.

## Shared URL state — `frontend/src/components/tableState.ts` (new)

```ts
const {
  page,
  pageSize,
  sort,
  filters,
  setPage,
  setPageSize,
  setSort,
  setFilter,
  clearFilters,
  hasFilters,
  queryString,
} = useTableParams({
  defaultPageSize: 50,
  defaultSort: { key: 'pickup_at', dir: 'asc' },
  filterKeys: ['status', 'date', 'attention'],
});
```

- URL keys: `page`, `limit`, `sort`, `dir`, plus the declared filter keys.
- Defaults (`page=1`, default page size, default sort) are omitted from the URL.
- Changing a filter, sort, or page size resets `page` to 1.
- `clearFilters()` removes filter keys, `sort`, `dir`, and `page`; it preserves unrelated params such as `scope`, `tab`, and `limit`.
- URL writes use `replace: true` so typing in a filter does not create history entries.
- `queryString` is the exact API query string, avoiding per-page query assembly.

## Backend contract

Add `sort` and `dir` query params to the paginated list endpoints, with allow-listed SQL expressions.

| Endpoint | Sortable fields | Existing default order |
|---|---|---|
| `GET /admin/users` | `name`, `email`, `status`, `created_at` | `u.name ASC` |
| `GET /admin/rides` | `pickup_at`, `passenger` → `p.name`, `area` → `a.name`, `driver` → `d.name`, `status` | `r.pickup_at ASC` |
| `GET /reimbursements` | `created_at`, `amount_cents`, `applicant` → `u.name`, `status` → priority case, `id` | review: pending-first then `id DESC`; payout: approved-first then `id DESC` |

Rules:

- `dir` accepts only `asc` or `desc`; anything else falls back to `ASC`.
- Unknown `sort` values fall back to the endpoint's existing order. No client-provided SQL identifier reaches the query.
- A stable `id` tie-breaker is appended where needed.
- Sorting applies to both paginated and legacy array responses.
- Count queries use the same filters.

Add a shared helper in `backend/src/util.rs`:

```rust
pub fn order_by(
    sort: Option<&str>,
    dir: Option<&str>,
    allowed: &[(&str, &str)],
    default: &str,
) -> String
```

Reimbursements also gains two filters:

- `q`: searches applicant name, payee name, and description.
- `status`: applies within the active scope (`review` or `payout`).

Members keeps `q`; Rides keeps `date`, `status`, and `attention`.

## Page wiring

- **Members** (`frontend/src/pages/admin/Members.tsx`): search moves into the toolbar; `name` and `email` become sortable; export remains available.
- **Rides** (`frontend/src/pages/admin/Rides.tsx`): status/date/attention move into the toolbar; pickup time, passenger, area, driver, and status become sortable; export stays in `PageHeader`.
- **Reimbursements** (`frontend/src/pages/admin/Reimbursements.tsx`): review/payout tabs stay; search and status filters join the toolbar; applicant, amount, submitted date, and status become sortable; export stays above the table.
- **Audit** (`frontend/src/pages/admin/Audit.tsx`): unchanged — newest-first keyset infinite scroll, existing filters, no column sorting.
- **Dashboard** (`frontend/src/pages/admin/Dashboard.tsx`): unchanged; local sorting is available later without backend work.

Loading remains page-level (`CardSkeleton`). Once loaded, the table always renders, including when the filtered result is empty.

## Styling and accessibility

Add CSS for:

- `.table-block`, `.table-toolbar`, `.table-filter`, `.table-toolbar-actions`
- `.th-sort`, `.sort-indicator`
- mobile toolbar wrapping and the mobile sort control

Accessibility requirements:

- `aria-sort` on sortable headers.
- A real `<button>` inside `<th>` with an accessible sort label.
- Filters use `aria-label` or visible checkbox labels.
- Existing `scope="col"` and responsive `data-label` behavior are preserved.

## Testing

### Frontend

- Local sorting, including null-last and stable ties.
- Controlled sorting: header click emits `{ key, dir }`; `aria-sort` follows the `sort` prop.
- Toolbar renders each filter type; changes emit values; clear button behavior.
- Mobile sort select.
- Row-click guard for interactive children.
- `useTableParams`: URL serialization, page reset, default omission, and `clearFilters`.

### Backend

- Allowed sort fields order correctly.
- Invalid `sort`/`dir` falls back safely.
- Filters and sorting combine correctly with pagination.
- Reimbursement search/status filters and scope-specific defaults.
- Existing defaults remain unchanged when no sort is sent.

## Out of scope (deliberate)

- Audit server-side sorting or replacing keyset pagination.
- Column visibility, bulk selection, expandable rows, and footer totals.
- Virtualized tables.
- The Roles permission matrix.
- New dependencies.

## Risks

- **Default mismatch**: frontend defaults must mirror backend defaults. The URL hook writes `sort`/`dir` explicitly once the user changes sorting, and defaults are covered by tests.
- **Reimbursement special ordering**: sorting replaces the pending/approved priority order; clearing sort restores the scope default.
- **Wrapper markup**: the table gains an outer `.table-block` wrapper for the toolbar. No current caller passes `className` to `UniversalTable`, so the existing style surface is unaffected.
- **URL bloat**: filter/search changes are immediately serialized. This matches current behavior; debouncing remains a page concern if needed later.

## Files expected to change

Frontend:

- `frontend/src/components/ui.tsx`
- `frontend/src/components/tableState.ts` (new)
- `frontend/src/styles.css`
- `frontend/src/components/ui.test.tsx`
- `frontend/src/pages/admin/Members.tsx`
- `frontend/src/pages/admin/Rides.tsx`
- `frontend/src/pages/admin/Reimbursements.tsx`
- relevant locale files for generic toolbar labels

Backend:

- `backend/src/util.rs`
- `backend/src/routes/admin.rs`
- `backend/src/routes/rides.rs`
- `backend/src/routes/community.rs`
- backend integration tests
