/**
 * Integration Tests for TransactionHistory Component
 *
 * These tests verify the complete integration of:
 * - TransactionHistory component
 * - useFilterState hook
 * - useTransactionHistory hook
 * - API service layer
 * - TanStack Query
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// Simple waitFor implementation
async function waitFor(callback: () => void, options: { timeout?: number } = {}) {
  const timeout = options.timeout || 3000;
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      callback();
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  // Final attempt
  callback();
}
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import TransactionHistory from '../pages/TransactionHistory';
import * as transactionHistoryApi from '../services/transactionHistoryApi';

// Mock the contract service to avoid initialization issues
vi.mock('../services/contracts', () => ({
  contractService: {
    initialize: vi.fn().mockResolvedValue(undefined),
    getContractId: vi.fn().mockReturnValue('CMOCK123'),
  },
}));

// Mock i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en' },
  }),
}));

// Mock useSocket hook
vi.mock('../hooks/useSocket', () => ({
  useSocket: () => ({
    socket: null,
    connected: false,
    isPollingFallback: false,
  }),
}));

// Mock ConnectionStatus component
vi.mock('../components/ConnectionStatus', () => ({
  ConnectionStatus: () => null,
}));

vi.mock('@stellar/design-system', () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
  Card: ({ children, addlClassName, ...props }: any) => (
    <div className={addlClassName} {...props}>
      {children}
    </div>
  ),
  Heading: ({ children }: any) => <h2>{children}</h2>,
  Text: ({ children }: any) => <p>{children}</p>,
  Input: ({ id, value, onChange, placeholder }: any) => (
    <input id={id} value={value} onChange={onChange} placeholder={placeholder} data-testid={id} />
  ),
  Select: ({ id, value, onChange, children }: any) => (
    <select id={id} value={value} onChange={onChange} data-testid={id}>
      {children}
    </select>
  ),
}));

/**
 * Helper function to render component with all required providers
 */
function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false, // Disable retries in tests
        gcTime: 0, // Disable caching in tests
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>{ui}</BrowserRouter>
    </QueryClientProvider>
  );
}

describe('TransactionHistory Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders component and fetches initial data', async () => {
    // Mock successful API response
    const mockData = {
      items: [
        {
          id: 'audit-1',
          kind: 'classic' as const,
          createdAt: '2024-01-15T10:00:00Z',
          status: 'confirmed',
          amount: '100',
          asset: 'XLM',
          actor: 'GABC123',
          txHash: 'abc123',
          label: 'Transaction Confirmed',
          badge: 'Classic',
        },
        {
          id: 'contract-1',
          kind: 'contract' as const,
          createdAt: '2024-01-15T11:00:00Z',
          status: 'indexed',
          amount: '1000',
          asset: 'USDC',
          actor: 'CABC123',
          txHash: 'def456',
          label: 'Transfer Event',
          badge: 'Contract Event',
        },
      ],
      hasMore: false,
      total: 2,
    };

    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue(mockData);

    const { getByText, getAllByText } = renderWithProviders(<TransactionHistory />);

    // Verify page title is rendered
    expect(getByText(/Transaction/)).toBeInTheDocument();
    expect(getByText(/History/)).toBeInTheDocument();

    // Wait for data to load
    await waitFor(() => {
      expect(getByText('Transaction Confirmed')).toBeInTheDocument();
    });

    // Verify both timeline items are displayed
    expect(getByText('Transaction Confirmed')).toBeInTheDocument();
    expect(getByText('Transfer Event')).toBeInTheDocument();

    // Verify badges are displayed
    expect(getByText('Classic')).toBeInTheDocument();
    expect(getByText('Contract Event')).toBeInTheDocument();

    // Verify status badges
    expect(getAllByText('confirmed')).toHaveLength(1);
    expect(getAllByText('indexed')).toHaveLength(1);
  });

  test('displays empty state when no data', async () => {
    // Mock empty API response
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [],
      hasMore: false,
      total: 0,
    });

    const { getByText } = renderWithProviders(<TransactionHistory />);

    // Wait for empty state to appear
    await waitFor(() => {
      expect(getByText('No transactions yet')).toBeInTheDocument();
    });

    // When no filters are active, should show default message
    expect(
      getByText(/Your payroll history will appear here once payments are sent/)
    ).toBeInTheDocument();
  });

  test('displays error state and retry button on API failure', async () => {
    // Mock API error
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockRejectedValue(
      new Error('Network error')
    );

    const { getByText, getByRole } = renderWithProviders(<TransactionHistory />);

    // Wait for error message to appear
    await waitFor(() => {
      expect(getByText(/Network error/)).toBeInTheDocument();
    });

    // Verify retry button is present
    const retryButton = getByRole('button', { name: /retry/i });
    expect(retryButton).toBeInTheDocument();
  });

  test('opens and closes filter panel', async () => {
    const user = userEvent.setup();

    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [],
      hasMore: false,
      total: 0,
    });

    const { getByRole, getByText, queryByText, getByPlaceholderText, getByLabelText } =
      renderWithProviders(<TransactionHistory />);

    // Find and click the Filters button
    const filtersButton = getByRole('button', { name: /Filters/i });
    await user.click(filtersButton);

    // Verify filter panel is displayed
    await waitFor(() => {
      expect(getByText('Advanced Filters')).toBeInTheDocument();
    });

    // Verify filter inputs are present
    expect(getByPlaceholderText(/Tx hash/)).toBeInTheDocument();
    expect(getByLabelText(/Status/i)).toBeInTheDocument();
    expect(getByPlaceholderText(/Name or wallet/)).toBeInTheDocument();

    // Close filter panel
    await user.click(filtersButton);

    // Verify filter panel is hidden
    await waitFor(() => {
      expect(queryByText('Advanced Filters')).not.toBeInTheDocument();
    });
  });

  test('updates filters and triggers debounced API call', async () => {
    const user = userEvent.setup();
    const fetchSpy = vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [],
      hasMore: false,
      total: 0,
    });

    const { getByRole, getByLabelText } = renderWithProviders(<TransactionHistory />);

    // Wait for initial load
    await waitFor(() => {
      expect(fetchSpy).toHaveBeenCalledTimes(1);
    });

    // Open filters
    const filtersButton = getByRole('button', { name: /Filters/i });
    await user.click(filtersButton);

    // Update status filter
    const statusSelect = getByLabelText(/Status/i);
    await user.selectOptions(statusSelect, 'confirmed');

    // Wait for debounced API call (300ms + processing time)
    await waitFor(
      () => {
        expect(fetchSpy).toHaveBeenCalledTimes(2);
      },
      { timeout: 1000 }
    );

    // Verify the API was called with the correct filter
    expect(fetchSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        filters: expect.objectContaining({
          status: 'confirmed',
        }),
      }),
      expect.any(Object) // AbortSignal is passed as second parameter
    );
  });

  test('displays active filter count badge', async () => {
    const user = userEvent.setup();

    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [],
      hasMore: false,
      total: 0,
    });

    const { getByRole, getByLabelText, getByPlaceholderText } = renderWithProviders(
      <TransactionHistory />
    );

    // Wait for initial render to complete
    await waitFor(() => {
      expect(getByRole('button', { name: /Filters/i })).toBeInTheDocument();
    });

    // Open filters
    const filtersButton = getByRole('button', { name: /Filters/i });
    await user.click(filtersButton);

    // Add a filter
    const statusSelect = getByLabelText(/Status/i);
    await user.selectOptions(statusSelect, 'confirmed');

    // Wait for filter count to update
    await waitFor(() => {
      expect(getByRole('button', { name: /Filters \(1\)/i })).toBeInTheDocument();
    });

    // Add another filter
    const assetInput = getByPlaceholderText(/USDC, XLM/);
    await user.type(assetInput, 'USDC');

    // Wait for filter count to update
    await waitFor(() => {
      expect(getByRole('button', { name: /Filters \(2\)/i })).toBeInTheDocument();
    });
  });

  test('resets all filters when Clear All is clicked', async () => {
    const user = userEvent.setup();
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [],
      hasMore: false,
      total: 0,
    });

    const { getByRole, getByLabelText, getByPlaceholderText } = renderWithProviders(
      <TransactionHistory />
    );

    // Open filters
    const filtersButton = getByRole('button', { name: /Filters/i });
    await user.click(filtersButton);

    // Add filters
    const statusSelect = getByLabelText(/Status/i);
    await user.selectOptions(statusSelect, 'confirmed');

    const assetInput = getByPlaceholderText(/USDC, XLM/);
    await user.type(assetInput, 'USDC');

    // Wait for filters to be applied
    await waitFor(() => {
      expect(getByRole('button', { name: /Filters \(2\)/i })).toBeInTheDocument();
    });

    // Click Clear All
    const clearButton = getByRole('button', { name: /Clear All/i });
    await user.click(clearButton);

    // Verify filters are reset
    await waitFor(() => {
      expect(getByRole('button', { name: /Filters$/i })).toBeInTheDocument();
    });

    // Verify filter inputs are cleared
    expect(statusSelect).toHaveValue('');
    expect(assetInput).toHaveValue('');
  });

  test('displays Load More button when hasMore is true', async () => {
    // Mock API response with more data available
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [
        {
          id: 'audit-1',
          kind: 'classic' as const,
          createdAt: '2024-01-15T10:00:00Z',
          status: 'confirmed',
          amount: '100',
          asset: 'XLM',
          actor: 'GABC123',
          txHash: 'abc123',
          label: 'Transaction Confirmed',
          badge: 'Classic',
        },
      ],
      hasMore: true,
      total: 50,
    });

    const { getByText, getByRole } = renderWithProviders(<TransactionHistory />);

    // Wait for data to load
    await waitFor(() => {
      expect(getByText('Transaction Confirmed')).toBeInTheDocument();
    });

    // Verify Load More button is displayed
    expect(getByRole('button', { name: /Load older records/i })).toBeInTheDocument();
  });

  test('hides Load More button when hasMore is false', async () => {
    // Mock API response with no more data
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [
        {
          id: 'audit-1',
          kind: 'classic' as const,
          createdAt: '2024-01-15T10:00:00Z',
          status: 'confirmed',
          amount: '100',
          asset: 'XLM',
          actor: 'GABC123',
          txHash: 'abc123',
          label: 'Transaction Confirmed',
          badge: 'Classic',
        },
      ],
      hasMore: false,
      total: 1,
    });

    const { getByText, queryByRole } = renderWithProviders(<TransactionHistory />);

    // Wait for data to load
    await waitFor(() => {
      expect(getByText('Transaction Confirmed')).toBeInTheDocument();
    });

    // Verify Load More button is NOT displayed
    expect(queryByRole('button', { name: /Load older records/i })).not.toBeInTheDocument();
  });

  test('displays contract event badge differently from classic badge', async () => {
    // Mock mixed data
    vi.spyOn(transactionHistoryApi, 'fetchHistoryPage').mockResolvedValue({
      items: [
        {
          id: 'audit-1',
          kind: 'classic' as const,
          createdAt: '2024-01-15T10:00:00Z',
          status: 'confirmed',
          amount: '100',
          asset: 'XLM',
          actor: 'GABC123',
          txHash: 'abc123',
          label: 'Transaction Confirmed',
          badge: 'Classic',
        },
        {
          id: 'contract-1',
          kind: 'contract' as const,
          createdAt: '2024-01-15T11:00:00Z',
          status: 'indexed',
          amount: '1000',
          asset: 'USDC',
          actor: 'CABC123',
          txHash: 'def456',
          label: 'Transfer Event',
          badge: 'Contract Event',
        },
      ],
      hasMore: false,
      total: 2,
    });

    const { getByText } = renderWithProviders(<TransactionHistory />);

    // Wait for data to load
    await waitFor(() => {
      expect(getByText('Classic')).toBeInTheDocument();
    });

    // Verify both badge types are present
    expect(getByText('Classic')).toBeInTheDocument();
    expect(getByText('Contract Event')).toBeInTheDocument();

    // Verify they are different
    const classicBadge = getByText('Classic');
    const contractBadge = getByText('Contract Event');
    expect(classicBadge.textContent).not.toBe(contractBadge.textContent);
  });
});
