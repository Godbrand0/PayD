import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ConnectedProvidersStatus } from '../ConnectedProvidersStatus';

const providersMock = [
  { provider: 'google', connected: true, email: 'user@example.com', connectedAt: '2024-01-01T12:00:00Z' },
  { provider: 'github', connected: false },
];

describe('ConnectedProvidersStatus', () => {
  it('renders connected and disconnected providers', () => {
    render(<ConnectedProvidersStatus providers={providersMock as any} />);
    expect(screen.getByText(/user@example.com/)).toBeInTheDocument();
    expect(screen.getByText(/Not connected/)).toBeInTheDocument();
  });

  it('shows check icon for connected provider', () => {
    render(<ConnectedProvidersStatus providers={providersMock as any} />);
    expect(document.querySelectorAll('svg[data-testid="connected-icon"]').length).toBeGreaterThan(0);
  });

  it('shows X icon for disconnected provider', () => {
    render(<ConnectedProvidersStatus providers={providersMock as any} />);
    expect(document.querySelectorAll('svg[data-testid="disconnected-icon"]').length).toBeGreaterThan(0);
  });

  it('renders custom className', () => {
    const { container } = render(<ConnectedProvidersStatus providers={providersMock as any} className="custom" />);
    expect(container.querySelector('.custom')).toBeInTheDocument();
  });
});
