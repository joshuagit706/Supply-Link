import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';

// ── Mocks ─────────────────────────────────────────────────────────────────────

const {
  mockRegisterAssembly,
  mockWalletAddress,
  mockToastLoading,
  mockToastSuccess,
  mockToastError,
  mockToastDismiss,
} = vi.hoisted(() => ({
  mockRegisterAssembly: vi.fn(),
  mockWalletAddress: { current: 'GOWNER123' as string | null },
  mockToastLoading: vi.fn().mockReturnValue('toast-id'),
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
  mockToastDismiss: vi.fn(),
}));

vi.mock('@/lib/stellar/client', () => ({
  registerAssembly: mockRegisterAssembly,
}));

vi.mock('@/lib/hooks/useToast', () => ({
  useToast: () => ({
    loading: mockToastLoading,
    success: mockToastSuccess,
    error: mockToastError,
    dismiss: mockToastDismiss,
  }),
}));

vi.mock('@/lib/state/store', () => ({
  useStore: (selector?: (s: { walletAddress: string | null }) => unknown) => {
    const state = { walletAddress: mockWalletAddress.current };
    return selector ? selector(state) : state;
  },
}));

// next/link stub
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) =>
    React.createElement('a', { href, ...rest }, children),
}));

import { AssemblyPanel } from '@/components/products/AssemblyPanel';
import type { ProductAssembly, Product } from '@/lib/types';

const MOCK_ASSEMBLY: ProductAssembly = {
  parentId: 'prod-parent',
  componentIds: ['comp-001', 'comp-002'],
  registeredBy: 'GOWNER123',
  registeredAt: 1712000000000,
  description: 'Assembled from two components',
};

const MOCK_PRODUCTS: Product[] = [
  {
    id: 'comp-001',
    name: 'Component A',
    origin: 'Germany',
    owner: 'GOWNER123',
    timestamp: 1710000000000,
    authorizedActors: [],
  },
  {
    id: 'comp-002',
    name: 'Component B',
    origin: 'France',
    owner: 'GOWNER123',
    timestamp: 1710000000000,
    authorizedActors: [],
  },
];

beforeEach(() => {
  vi.clearAllMocks();
  mockWalletAddress.current = 'GOWNER123';
});

describe('AssemblyPanel', () => {
  it('renders "no assembly" message when no assembly provided', () => {
    render(<AssemblyPanel productId="prod-001" />);
    expect(screen.getByText(/no assembly relationship registered/i)).toBeInTheDocument();
  });

  it('renders component IDs when assembly is provided', () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
      />,
    );
    expect(screen.getByText('comp-001')).toBeInTheDocument();
    expect(screen.getByText('comp-002')).toBeInTheDocument();
  });

  it('renders assembly description', () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
      />,
    );
    expect(screen.getByText('Assembled from two components')).toBeInTheDocument();
  });

  it('renders component count badge', () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
      />,
    );
    expect(screen.getByText(/2 components/i)).toBeInTheDocument();
  });

  it('shows "View provenance" links for each component', () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
      />,
    );
    const links = screen.getAllByText(/view provenance/i);
    expect(links).toHaveLength(2);
  });

  it('shows register button for owner when no assembly exists', () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    expect(screen.getByText(/register assembly/i)).toBeInTheDocument();
  });

  it('does not show register button for non-owner', () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={false}
      />,
    );
    expect(screen.queryByText(/register assembly/i)).not.toBeInTheDocument();
  });

  it('opens register form when register button is clicked', async () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    expect(screen.getByText(/select component products/i)).toBeInTheDocument();
  });

  it('shows component checkboxes in register form', async () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    expect(screen.getByText('Component A')).toBeInTheDocument();
    expect(screen.getByText('Component B')).toBeInTheDocument();
  });

  it('calls registerAssembly with selected components on submit', async () => {
    mockRegisterAssembly.mockResolvedValue('mock_tx_123');
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));

    // Select first component
    const checkboxes = screen.getAllByRole('checkbox');
    await userEvent.click(checkboxes[0]);

    await userEvent.click(screen.getByRole('button', { name: /save assembly/i }));

    await waitFor(() => {
      expect(mockRegisterAssembly).toHaveBeenCalledWith(
        'prod-001',
        ['comp-001'],
        '',
        'GOWNER123',
      );
    });
  });

  it('shows success toast after successful registration', async () => {
    mockRegisterAssembly.mockResolvedValue('mock_tx_123');
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    const checkboxes = screen.getAllByRole('checkbox');
    await userEvent.click(checkboxes[0]);
    await userEvent.click(screen.getByRole('button', { name: /save assembly/i }));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith(
        'Assembly registered',
        expect.any(String),
      );
    });
  });

  it('shows error toast when wallet not connected', async () => {
    mockWalletAddress.current = null;
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    const checkboxes = screen.getAllByRole('checkbox');
    await userEvent.click(checkboxes[0]);
    await userEvent.click(screen.getByRole('button', { name: /save assembly/i }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith(
        'Wallet not connected',
        expect.any(String),
      );
    });
  });

  it('shows error toast when no components selected', async () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    // Don't select any component
    await userEvent.click(screen.getByRole('button', { name: /save assembly/i }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith(
        'No components selected',
        expect.any(String),
      );
    });
  });

  it('collapses panel when header is clicked', async () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
      />,
    );
    // Initially expanded — component IDs visible
    expect(screen.getByText('comp-001')).toBeInTheDocument();

    // Click header to collapse
    await userEvent.click(screen.getByRole('button', { name: /assembly structure/i }));
    expect(screen.queryByText('comp-001')).not.toBeInTheDocument();
  });

  it('shows update assembly button for owner when assembly exists', () => {
    render(
      <AssemblyPanel
        productId="prod-parent"
        assembly={MOCK_ASSEMBLY}
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    expect(screen.getByText(/update assembly/i)).toBeInTheDocument();
  });

  it('cancels form when cancel button is clicked', async () => {
    render(
      <AssemblyPanel
        productId="prod-001"
        allProducts={MOCK_PRODUCTS}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/register assembly/i));
    expect(screen.getByText(/select component products/i)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(screen.queryByText(/select component products/i)).not.toBeInTheDocument();
  });
});
