import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Trans } from 'react-i18next';
import i18n from './i18n';

describe('Trans with a named tag', () => {
  it('renders the alternative-transport notice with a working link', async () => {
    await i18n.changeLanguage('en');
    // The link text comes from the translation string, so the <a> is empty here on purpose.
    // eslint-disable-next-line jsx-a11y/anchor-has-content
    render(<Trans t={i18n.getFixedT('en', 'rideDetail')} i18nKey="altTransportNotice" components={{ receipt: <a href="/reimbursements/new?ride=7" /> }} />);
    const link = screen.getByRole('link', { name: 'upload your Grab receipt' });
    expect(link).toHaveAttribute('href', '/reimbursements/new?ride=7');
    expect(document.body.textContent).not.toContain('[object Object]');
  });
});
