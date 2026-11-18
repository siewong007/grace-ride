import React, { ReactNode } from 'react';
import { Link } from 'react-router-dom';

export interface CardProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'onClick'> {
  variant?: 'default' | 'flat' | 'link';
  padding?: 'sm' | 'md' | 'lg';
  to?: string;
  onClick?: React.MouseEventHandler<HTMLDivElement | HTMLAnchorElement>;
  children: ReactNode;
}

/**
 * Container card wrapping responsive surface styles and padding variants.
 */
export function Card({
  variant = 'default',
  padding = 'md',
  to,
  className = '',
  children,
  onClick,
  ...rest
}: CardProps) {
  const classes = [
    'card',
    variant !== 'default' ? variant : '',
    padding === 'sm' ? 'tight' : padding === 'lg' ? 'pad-lg' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  if (to) {
    return (
      <Link
        to={to}
        className={classes}
        onClick={onClick}
        id={rest.id}
        style={rest.style}
        title={rest.title}
        tabIndex={rest.tabIndex}
        role={rest.role}
        aria-label={rest['aria-label']}
        aria-labelledby={rest['aria-labelledby']}
        aria-describedby={rest['aria-describedby']}
      >
        {children}
      </Link>
    );
  }

  return (
    <div
      className={classes}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          onClick?.(e as unknown as React.MouseEvent<HTMLDivElement>);
        }
      }}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      {...rest}
    >
      {children}
    </div>
  );
}

export interface StatProps {
  label: ReactNode;
  value: ReactNode;
  icon?: ReactNode;
  tone?: 'normal' | 'warn' | 'bad' | 'success';
  to?: string;
  className?: string;
}

/**
 * Metric summary tile for dashboards with optional navigation link and tone.
 */
export function Stat({
  label,
  value,
  tone = 'normal',
  to,
  className = '',
}: StatProps) {
  const toneCls = tone !== 'normal' ? tone : '';
  const classes = ['card', to ? 'link' : '', 'stat', toneCls, className].filter(Boolean).join(' ');

  const content = (
    <>
      <span className="label">{label}</span>
      <span className="num">{value}</span>
    </>
  );

  if (to) {
    return (
      <Link to={to} className={classes}>
        {content}
      </Link>
    );
  }

  return <div className={classes}>{content}</div>;
}

export interface KeyValueItem {
  label: ReactNode;
  value: ReactNode;
}

export interface KeyValueProps {
  items: KeyValueItem[];
  className?: string;
}

/**
 * Structured key-value definition list replacing ad-hoc dl.kv pairs.
 */
export function KeyValue({ items, className = '' }: KeyValueProps) {
  return (
    <dl className={`kv ${className}`.trim()}>
      {items.map((item, idx) => (
        <React.Fragment key={idx}>
          <dt>{item.label}</dt>
          <dd>{item.value}</dd>
        </React.Fragment>
      ))}
    </dl>
  );
}
