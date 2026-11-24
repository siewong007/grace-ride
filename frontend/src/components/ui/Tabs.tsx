import React, { ReactNode, useCallback, useId, useLayoutEffect, useRef, useState } from 'react';

export interface TabItem {
  value: string;
  label: ReactNode;
  count?: number;
  icon?: ReactNode;
}

export interface TabsProps {
  value: string;
  onChange: (value: string) => void;
  items: TabItem[];
  ariaLabel?: string;
  className?: string;
}

/**
 * Accessible tablist supporting keyboard arrow navigation and responsive phone scrolling.
 */
export function Tabs({ value, onChange, items, ariaLabel, className = '' }: TabsProps) {
  const idPrefix = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const btnRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const [indicator, setIndicator] = useState<{ left: number; width: number }>({ left: 0, width: 0 });

  const measure = useCallback(() => {
    const btn = btnRefs.current[value];
    if (!btn || btn.offsetWidth === 0) {
      setIndicator((cur) => (cur.width === 0 ? cur : { left: 0, width: 0 }));
      return;
    }
    const left = btn.offsetLeft;
    const width = btn.offsetWidth;
    setIndicator((cur) => (cur.left === left && cur.width === width ? cur : { left, width }));
  }, [value]);

  useLayoutEffect(() => {
    measure();
    const listEl = listRef.current;
    if (!listEl) return;
    let ro: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(measure);
      ro.observe(listEl);
    }
    window.addEventListener('resize', measure);
    return () => {
      ro?.disconnect();
      window.removeEventListener('resize', measure);
    };
  }, [measure, items.length]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    let nextIndex: number;
    if (e.key === 'ArrowRight') {
      nextIndex = (index + 1) % items.length;
    } else if (e.key === 'ArrowLeft') {
      nextIndex = (index - 1 + items.length) % items.length;
    } else if (e.key === 'Home') {
      nextIndex = 0;
    } else if (e.key === 'End') {
      nextIndex = items.length - 1;
    } else {
      return;
    }
    e.preventDefault();
    onChange(items[nextIndex].value);
    const btn = document.getElementById(`${idPrefix}-tab-${items[nextIndex].value}`);
    btn?.focus();
  };

  const hasIndicator = indicator.width > 0;

  return (
    <div
      ref={listRef}
      className={`tabs${hasIndicator ? ' has-indicator' : ''} ${className}`.trim()}
      role="tablist"
      aria-label={ariaLabel}
    >
      {items.map((item, idx) => {
        const isSelected = item.value === value;
        return (
          <button
            key={item.value}
            ref={(el) => {
              btnRefs.current[item.value] = el;
            }}
            id={`${idPrefix}-tab-${item.value}`}
            type="button"
            role="tab"
            aria-selected={isSelected}
            tabIndex={isSelected ? 0 : -1}
            className={isSelected ? 'on' : ''}
            onClick={() => onChange(item.value)}
            onKeyDown={(e) => handleKeyDown(e, idx)}
          >
            {item.icon && <span className="tab-icon" aria-hidden="true">{item.icon}</span>}
            <span>{item.label}</span>
            {item.count != null && (
              <span className="tab-count" aria-label={`(${item.count})`}>
                {item.count}
              </span>
            )}
          </button>
        );
      })}
      {hasIndicator && (
        <span
          className="tabs-indicator"
          aria-hidden="true"
          style={{ width: `${indicator.width}px`, transform: `translateX(${indicator.left}px)` }}
        />
      )}
    </div>
  );
}
