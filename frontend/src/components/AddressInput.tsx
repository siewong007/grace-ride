// Address field with place suggestions as you type (OpenStreetMap data via /api/places).
// Free text still works: suggestions only help members type a full, findable address.
import { KeyboardEvent, ReactNode, useEffect, useId, useRef, useState } from 'react';
import { MapPin } from 'lucide-react';
import { api } from '../api';

export interface Place {
  label: string;
  lat: number;
  lng: number;
}

const DEBOUNCE_MS = 300;
const MIN_CHARS = 3;

interface Props {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  onBlur?: (value: string) => void;
  hint?: ReactNode;
  placeholder?: string;
  required?: boolean;
  invalid?: boolean;
}

/** Combobox (WAI-ARIA list autocomplete): arrows move, Enter picks, Escape closes. */
export function AddressInput({ id, label, value, onChange, onBlur, hint, placeholder, required, invalid }: Props) {
  const listId = useId();
  const [options, setOptions] = useState<Place[]>([]);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  // Only text the person typed triggers a search; a picked suggestion doesn't.
  const [query, setQuery] = useState<string | null>(null);
  const seq = useRef(0);
  const wrapRef = useRef<HTMLDivElement>(null);
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (query === null) return;
    const q = query.trim();
    if (q.length < MIN_CHARS) {
      setOptions([]);
      return;
    }
    const mine = ++seq.current;
    const timer = window.setTimeout(() => {
      api<Place[]>(`/places?q=${encodeURIComponent(q)}`)
        .then((hits) => {
          if (mine !== seq.current) return; // a newer keystroke won
          setOptions(hits);
          setActive(-1);
        })
        .catch(() => mine === seq.current && setOptions([]));
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  const showList = open && options.length > 0;

  // On phones the field can sit low, where the sticky submit bar and tab bar would cover
  // the list: bring the field up so the suggestions have room.
  useEffect(() => {
    if (!showList || !popRef.current || !wrapRef.current) return;
    const covered = popRef.current.getBoundingClientRect().bottom > window.innerHeight - 160;
    if (covered) wrapRef.current.scrollIntoView?.({ block: 'start', behavior: 'smooth' });
  }, [showList]);

  const pick = (p: Place) => {
    seq.current++; // drop any search still in flight
    setQuery(null);
    setOptions([]);
    setOpen(false);
    setActive(-1);
    onChange(p.label);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown' && options.length) {
      e.preventDefault();
      setOpen(true);
      setActive((i) => (i + 1) % options.length);
    } else if (e.key === 'ArrowUp' && options.length) {
      e.preventDefault();
      setOpen(true);
      setActive((i) => (i <= 0 ? options.length - 1 : i - 1));
    } else if (e.key === 'Enter' && showList && active >= 0) {
      e.preventDefault(); // pick, don't submit the form
      pick(options[active]);
    } else if (e.key === 'Escape' && showList) {
      e.preventDefault();
      setOpen(false);
      setActive(-1);
    }
  };

  const optionId = (i: number) => `${listId}-opt-${i}`;

  return (
    <div className="field">
      <label htmlFor={id} className="field-label">{label}</label>
      <div className="address-ac" ref={wrapRef}>
        <input
          id={id}
          className="input"
          type="text"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={showList}
          aria-controls={listId}
          aria-activedescendant={showList && active >= 0 ? optionId(active) : undefined}
          aria-invalid={invalid || undefined}
          autoComplete="off"
          required={required}
          value={value}
          placeholder={placeholder}
          onChange={(e) => {
            onChange(e.target.value);
            setQuery(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onBlur={(e) => {
            setOpen(false);
            setActive(-1);
            onBlur?.(e.target.value);
          }}
          onKeyDown={onKeyDown}
        />
        <div className="address-ac-pop" ref={popRef} hidden={!showList}>
          <ul id={listId} role="listbox" aria-label={label} className="address-ac-list">
            {options.map((p, i) => {
              const [head, ...rest] = p.label.split(', ');
              return (
                <li
                  key={p.label}
                  id={optionId(i)}
                  role="option"
                  aria-selected={i === active}
                  className={i === active ? 'address-ac-option is-active' : 'address-ac-option'}
                  // mousedown, not click: keeps focus in the field so blur doesn't close the list first
                  onMouseDown={(e) => {
                    e.preventDefault();
                    pick(p);
                  }}
                  onMouseEnter={() => setActive(i)}
                >
                  <MapPin size={16} aria-hidden="true" className="address-ac-icon" />
                  <span className="address-ac-text">
                    <span className="address-ac-head">{head}</span>
                    {rest.length > 0 && <span className="address-ac-rest">{rest.join(', ')}</span>}
                  </span>
                </li>
              );
            })}
          </ul>
          <div className="address-ac-credit">© OpenStreetMap contributors</div>
        </div>
      </div>
      {hint && <small className="hint">{hint}</small>}
    </div>
  );
}
