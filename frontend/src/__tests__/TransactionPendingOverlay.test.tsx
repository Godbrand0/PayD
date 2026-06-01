/**
 * Unit Tests for TransactionPendingOverlay Component
 */

import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TransactionPendingOverlay } from '../components/TransactionPendingOverlay';

describe('TransactionPendingOverlay', () => {
  test('renders nothing when isVisible is false', () => {
    render(<TransactionPendingOverlay isVisible={false} />);
    expect(screen.queryByText('Broadcasted to Stellar')).not.toBeInTheDocument();
  });

  test('displays pending state with default message', () => {
    render(<TransactionPendingOverlay isVisible={true} status="pending" />);

    expect(screen.getByText('Broadcasted to Stellar')).toBeInTheDocument();
    expect(
      screen.getByText('Your transaction is being processed on-chain. This may take a few seconds.')
    ).toBeInTheDocument();
    expect(screen.getByText('Settling on Stellar network...')).toBeInTheDocument();
  });

  test('displays pending state with custom message', () => {
    render(
      <TransactionPendingOverlay
        isVisible={true}
        status="pending"
        message="Custom Pending Message"
        subMessage="Custom sub message"
      />
    );

    expect(screen.getByText('Custom Pending Message')).toBeInTheDocument();
    expect(screen.getByText('Custom sub message')).toBeInTheDocument();
  });

  test('displays success state with default message', () => {
    render(<TransactionPendingOverlay isVisible={true} status="success" />);

    expect(screen.getByText('Transaction Confirmed')).toBeInTheDocument();
    expect(
      screen.getByText('Your transaction has been successfully processed.')
    ).toBeInTheDocument();
  });

  test('displays success state with txHash and explorer link', () => {
    const txHash = 'abc123def456789';
    render(<TransactionPendingOverlay isVisible={true} status="success" txHash={txHash} />);

    expect(screen.getByText('Transaction Confirmed')).toBeInTheDocument();

    const explorerLink = screen.getByLabelText('View transaction on explorer');
    expect(explorerLink).toBeInTheDocument();
    expect(explorerLink).toHaveAttribute('href', expect.stringContaining(txHash));

    expect(screen.getByText('Transaction Hash')).toBeInTheDocument();
  });

  test('displays error state with default message', () => {
    render(<TransactionPendingOverlay isVisible={true} status="error" />);

    expect(screen.getByText('Transaction Failed')).toBeInTheDocument();
    expect(screen.getByText('There was an issue processing your transaction.')).toBeInTheDocument();
  });

  test('shows dismiss button on success state', () => {
    const onDismiss = vi.fn();
    render(<TransactionPendingOverlay isVisible={true} status="success" onDismiss={onDismiss} />);

    const dismissButton = screen.getByText('Dismiss');
    expect(dismissButton).toBeInTheDocument();

    fireEvent.click(dismissButton);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  test('shows dismiss button on error state', () => {
    const onDismiss = vi.fn();
    render(<TransactionPendingOverlay isVisible={true} status="error" onDismiss={onDismiss} />);

    const dismissButton = screen.getByText('Dismiss');
    expect(dismissButton).toBeInTheDocument();

    fireEvent.click(dismissButton);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  test('does not show dismiss button during pending state', () => {
    const onDismiss = vi.fn();
    render(<TransactionPendingOverlay isVisible={true} status="pending" onDismiss={onDismiss} />);

    expect(screen.queryByText('Dismiss')).not.toBeInTheDocument();
  });

  test('has proper accessibility attributes', () => {
    render(<TransactionPendingOverlay isVisible={true} status="pending" />);

    const overlay = screen.getByRole('dialog');
    expect(overlay).toBeInTheDocument();
    expect(overlay).toHaveAttribute('aria-live', 'polite');
    expect(overlay).toHaveAttribute('aria-label', 'Broadcasted to Stellar');
  });

  test('truncates long txHash correctly', () => {
    const longTxHash = 'a'.repeat(64);
    render(<TransactionPendingOverlay isVisible={true} status="success" txHash={longTxHash} />);

    const truncatedHash = screen.getByText(/^aaaaaaaaaaaa/);
    expect(truncatedHash).toBeInTheDocument();
  });
});
