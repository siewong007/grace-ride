import React, { ReactNode, useId } from 'react';

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

  return (
    <div className={`tabs ${className}`.trim()} role="tablist" aria-label={ariaLabel}>
      {items.map((item, idx) => {
        const isSelected = item.value === value;
        return (
          <button
            key={item.value}
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
    </div>
  );
}
