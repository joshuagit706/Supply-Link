import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';

// ── Mocks ─────────────────────────────────────────────────────────────────────

const {
  mockRegisterWarranty,
  mockFileWarrantyClaim,
  mockUpdateClaimStatus,
  mockVoidWarranty,
  mockWalletAddress,
  mockToastLoading,
  mockToastSuccess,
  mockToastError,
  mockToastDismiss,
} = vi.hoisted(() => ({
  mockRegisterWarranty: vi.fn(),
  mockFileWarrantyClaim: vi.fn(),
  mockUpdateClaimStatus: vi.fn(),
  mockVoidWarranty: vi.fn(),
  mockWalletAddress: { current: 'GOWNER123' as string | null },
  mockToastLoading: vi.fn().mockReturnValue('toast-id'),
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
  mockToastDismiss: vi.fn(),
}));

vi.mock('@/lib/stellar/client', () => ({
  registerWarranty: mockRegisterWarranty,
  fileWarrantyClaim: mockFileWarrantyClaim,
  updateClaimStatus: mockUpdateClaimStatus,
  voidWarranty: mockVoidWarranty,
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

import { WarrantyPanel } from '@/components/products/WarrantyPanel';
import type { WarrantyInfo, WarrantyClaim } from '@/lib/types';

// Product registered 1 year ago
const ONE_YEAR_AGO = Date.now() - 365 * 24 * 3600 * 1000;
const PRODUCT_TIMESTAMP = ONE_YEAR_AGO;

const ACTIVE_WARRANTY: WarrantyInfo = {
  productId: 'prod-001',
  durationSeconds: 2 * 365 * 24 * 3600, // 2 years
  issuer: 'GOWNER123',
  issuedAt: PRODUCT_TIMESTAMP,
  terms: 'Full replacement within 2 years.',
  termsRef: 'ipfs://QmWarrantyDoc',
  voided: false,
  voidedAt: 0,
};

const EXPIRED_WARRANTY: WarrantyInfo = {
  ...ACTIVE_WARRANTY,
  durationSeconds: 1, // 1 second — already expired
};

const VOIDED_WARRANTY: WarrantyInfo = {
  ...ACTIVE_WARRANTY,
  voided: true,
  voidedAt: Date.now() - 1000,
};

const LIFETIME_WARRANTY: WarrantyInfo = {
  ...ACTIVE_WARRANTY,
  durationSeconds: 0,
  terms: 'Lifetime warranty.',
};

const PENDING_CLAIM: WarrantyClaim = {
  claimId: 'claim-001',
  productId: 'prod-001',
  claimant: 'GCUSTOMER123',
  filedAt: Date.now() - 86400000,
  description: 'Product stopped working.',
  proofRef: 'ipfs://QmProof',
  status: 'Pending',
  updatedAt: Date.now() - 86400000,
};

beforeEach(() => {
  vi.clearAllMocks();
  mockWalletAddress.current = 'GOWNER123';
});

describe('WarrantyPanel', () => {
  // ── No warranty ─────────────────────────────────────────────────────────────

  it('renders "no warranty" message when no warranty provided', () => {
    render(<WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} />);
    expect(screen.getByText(/no warranty registered/i)).toBeInTheDocument();
  });

  it('shows register button for owner when no warranty', () => {
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={true} />,
    );
    expect(screen.getByText(/register warranty/i)).toBeInTheDocument();
  });

  it('does not show register button for non-owner', () => {
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={false} />,
    );
    expect(screen.queryByText(/register warranty/i)).not.toBeInTheDocument();
  });

  // ── Active warranty ──────────────────────────────────────────────────────────

  it('renders Active badge for active warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
      />,
    );
    expect(screen.getByText('Active')).toBeInTheDocument();
  });

  it('renders warranty terms', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
      />,
    );
    expect(screen.getByText('Full replacement within 2 years.')).toBeInTheDocument();
  });

  it('renders warranty duration', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
      />,
    );
    expect(screen.getByText(/2 years/i)).toBeInTheDocument();
  });

  it('renders "Lifetime" for zero-duration warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={LIFETIME_WARRANTY}
      />,
    );
    expect(screen.getByText('Lifetime')).toBeInTheDocument();
  });

  it('renders document reference link', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
      />,
    );
    const link = screen.getByText('ipfs://QmWarrantyDoc');
    expect(link.closest('a')).toHaveAttribute('href', 'ipfs://QmWarrantyDoc');
  });

  // ── Expired warranty ─────────────────────────────────────────────────────────

  it('renders Expired badge for expired warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={EXPIRED_WARRANTY}
      />,
    );
    expect(screen.getByText('Expired')).toBeInTheDocument();
  });

  // ── Voided warranty ──────────────────────────────────────────────────────────

  it('renders Voided badge for voided warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={VOIDED_WARRANTY}
      />,
    );
    expect(screen.getByText('Voided')).toBeInTheDocument();
  });

  it('does not show void button when warranty is already voided', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={VOIDED_WARRANTY}
        isOwner={true}
      />,
    );
    expect(screen.queryByText(/void warranty/i)).not.toBeInTheDocument();
  });

  // ── Register warranty form ───────────────────────────────────────────────────

  it('opens register form when register button clicked', async () => {
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={true} />,
    );
    await userEvent.click(screen.getByText(/register warranty/i));
    expect(screen.getByLabelText(/duration/i)).toBeInTheDocument();
  });

  it('calls registerWarranty on form submit', async () => {
    mockRegisterWarranty.mockResolvedValue('mock_tx_warranty');
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={true} />,
    );
    await userEvent.click(screen.getByText(/register warranty/i));
    await userEvent.click(screen.getByRole('button', { name: /register warranty/i }));

    await waitFor(() => {
      expect(mockRegisterWarranty).toHaveBeenCalledWith(
        'prod-001',
        expect.any(Number),
        expect.any(String),
        expect.any(String),
        'GOWNER123',
      );
    });
  });

  it('shows success toast after warranty registration', async () => {
    mockRegisterWarranty.mockResolvedValue('mock_tx_warranty');
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={true} />,
    );
    await userEvent.click(screen.getByText(/register warranty/i));
    await userEvent.click(screen.getByRole('button', { name: /register warranty/i }));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Warranty registered', expect.any(String));
    });
  });

  it('shows error toast when wallet not connected during registration', async () => {
    mockWalletAddress.current = null;
    render(
      <WarrantyPanel productId="prod-001" productTimestamp={PRODUCT_TIMESTAMP} isOwner={true} />,
    );
    await userEvent.click(screen.getByText(/register warranty/i));
    await userEvent.click(screen.getByRole('button', { name: /register warranty/i }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Wallet not connected', expect.any(String));
    });
  });

  // ── Void warranty ────────────────────────────────────────────────────────────

  it('shows void button for owner with active warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        isOwner={true}
      />,
    );
    expect(screen.getByText(/void warranty/i)).toBeInTheDocument();
  });

  it('calls voidWarranty when void button clicked', async () => {
    mockVoidWarranty.mockResolvedValue('mock_tx_void');
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByText(/void warranty/i));

    await waitFor(() => {
      expect(mockVoidWarranty).toHaveBeenCalledWith('prod-001', 'GOWNER123');
    });
  });

  // ── Claims ───────────────────────────────────────────────────────────────────

  it('renders "no claims" message when no claims', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    expect(screen.getByText(/no claims filed yet/i)).toBeInTheDocument();
  });

  it('renders existing claims', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[PENDING_CLAIM]}
      />,
    );
    expect(screen.getByText('Product stopped working.')).toBeInTheDocument();
    expect(screen.getByText('Pending')).toBeInTheDocument();
  });

  it('shows "File claim" button for active warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    expect(screen.getByText(/file claim/i)).toBeInTheDocument();
  });

  it('does not show "File claim" for voided warranty', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={VOIDED_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    expect(screen.queryByText(/file claim/i)).not.toBeInTheDocument();
  });

  it('opens claim form when "File claim" clicked', async () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    await userEvent.click(screen.getByText(/file claim/i));
    expect(screen.getByLabelText(/issue description/i)).toBeInTheDocument();
  });

  it('calls fileWarrantyClaim on claim form submit', async () => {
    mockFileWarrantyClaim.mockResolvedValue('mock_tx_claim');
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    await userEvent.click(screen.getByText(/file claim/i));
    await userEvent.type(
      screen.getByLabelText(/issue description/i),
      'Screen cracked after normal use',
    );
    await userEvent.click(screen.getByRole('button', { name: /file claim/i }));

    await waitFor(() => {
      expect(mockFileWarrantyClaim).toHaveBeenCalledWith(
        'prod-001',
        expect.any(String),
        'Screen cracked after normal use',
        '',
        'GOWNER123',
      );
    });
  });

  it('shows error when description is empty on claim submit', async () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[]}
      />,
    );
    await userEvent.click(screen.getByText(/file claim/i));
    await userEvent.click(screen.getByRole('button', { name: /file claim/i }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Description required', expect.any(String));
    });
  });

  it('shows status update buttons for owner on pending claims', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[PENDING_CLAIM]}
        isOwner={true}
      />,
    );
    expect(screen.getByRole('button', { name: /approved/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /rejected/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /resolved/i })).toBeInTheDocument();
  });

  it('calls updateClaimStatus when owner approves a claim', async () => {
    mockUpdateClaimStatus.mockResolvedValue('mock_tx_status');
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[PENDING_CLAIM]}
        isOwner={true}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /approved/i }));

    await waitFor(() => {
      expect(mockUpdateClaimStatus).toHaveBeenCalledWith(
        'prod-001',
        'claim-001',
        'Approved',
        'GOWNER123',
      );
    });
  });

  // ── Collapse ─────────────────────────────────────────────────────────────────

  it('collapses panel when header clicked', async () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
      />,
    );
    expect(screen.getByText('Full replacement within 2 years.')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /warranty/i }));
    expect(screen.queryByText('Full replacement within 2 years.')).not.toBeInTheDocument();
  });

  // ── Claims count ─────────────────────────────────────────────────────────────

  it('renders claims count in section header', () => {
    render(
      <WarrantyPanel
        productId="prod-001"
        productTimestamp={PRODUCT_TIMESTAMP}
        warranty={ACTIVE_WARRANTY}
        warrantyClaims={[PENDING_CLAIM]}
      />,
    );
    expect(screen.getByText(/claims \(1\)/i)).toBeInTheDocument();
  });
});
