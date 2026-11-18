import React, { forwardRef, ReactNode } from 'react';
import { Link } from 'react-router-dom';

type SharedActionAttributes = Omit<
  React.ButtonHTMLAttributes<HTMLButtonElement>,
  'onClick'
> & {
  onClick?: React.MouseEventHandler<HTMLButtonElement | HTMLAnchorElement>;
  target?: string;
  rel?: string;
};

export interface ButtonProps extends SharedActionAttributes {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  block?: boolean;
  loading?: boolean;
  icon?: ReactNode;
  to?: string;
}

/**
 * Standard button primitive with variants, loading state, icon support, and link routing.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = 'secondary',
    size = 'md',
    block = false,
    loading = false,
    icon,
    to,
    type = 'button',
    disabled,
    className = '',
    children,
    onClick,
    target,
    rel,
    form,
    formAction,
    formEncType,
    formMethod,
    formNoValidate,
    formTarget,
    name,
    value,
    ...rest
  }: ButtonProps,
  ref,
) {
  const classes = [
    'btn',
    variant !== 'secondary' ? variant : '',
    size !== 'md' ? size : '',
    block ? 'block' : '',
    loading ? 'loading' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  const content = (
    <>
      {loading ? <span className="btn-spinner" aria-hidden="true" /> : icon}
      {children}
    </>
  );

  if (to && !disabled && !loading) {
    return (
      <Link
        to={to}
        className={classes}
        onClick={onClick}
        target={target}
        rel={rel}
        id={rest.id}
        style={rest.style}
        title={rest.title}
        tabIndex={rest.tabIndex}
        role={rest.role}
        aria-label={rest['aria-label']}
        aria-labelledby={rest['aria-labelledby']}
        aria-describedby={rest['aria-describedby']}
        aria-current={rest['aria-current']}
        aria-expanded={rest['aria-expanded']}
        aria-haspopup={rest['aria-haspopup']}
      >
        {content}
      </Link>
    );
  }

  return (
    <button
      ref={ref}
      type={type}
      className={classes}
      disabled={disabled || loading}
      aria-busy={loading ? 'true' : undefined}
      onClick={onClick}
      form={form}
      formAction={formAction}
      formEncType={formEncType}
      formMethod={formMethod}
      formNoValidate={formNoValidate}
      formTarget={formTarget}
      name={name}
      value={value}
      {...rest}
    >
      {content}
    </button>
  );
});

export interface IconButtonProps extends SharedActionAttributes {
  label: string;
  icon: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md';
  to?: string;
  children?: ReactNode;
}

/**
 * Accessible 48px square icon button with a required aria-label.
 */
export function IconButton({
  label,
  icon,
  variant = 'secondary',
  size = 'md',
  to,
  type = 'button',
  disabled,
  className = '',
  children,
  onClick,
  target,
  rel,
  form,
  formAction,
  formEncType,
  formMethod,
  formNoValidate,
  formTarget,
  name,
  value,
  ...rest
}: IconButtonProps) {
  const classes = [
    'icon-btn',
    variant !== 'secondary' ? variant : '',
    size === 'sm' ? 'sm' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  if (to && !disabled) {
    return (
      <Link
        to={to}
        className={classes}
        aria-label={label}
        onClick={onClick}
        target={target}
        rel={rel}
        id={rest.id}
        style={rest.style}
        title={rest.title}
        tabIndex={rest.tabIndex}
        role={rest.role}
        aria-current={rest['aria-current']}
        aria-expanded={rest['aria-expanded']}
        aria-haspopup={rest['aria-haspopup']}
      >
        {icon}
        {children}
      </Link>
    );
  }

  return (
    <button
      type={type}
      className={classes}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      form={form}
      formAction={formAction}
      formEncType={formEncType}
      formMethod={formMethod}
      formNoValidate={formNoValidate}
      formTarget={formTarget}
      name={name}
      value={value}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}
