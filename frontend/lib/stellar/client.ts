import { isConnected, signTransaction, getAddress } from '@stellar/freighter-api';

export type StellarNetwork = 'testnet' | 'mainnet';

interface NetworkConfig {
  passphrase: string;
  rpcUrl: string;
  name: string;
}

const NETWORKS: Record<StellarNetwork, NetworkConfig> = {
  testnet: {
    passphrase: 'Test SDF Network ; September 2015',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    name: 'Testnet',
  },
  mainnet: {
    passphrase: 'Public Global Stellar Network ; September 2015',
    rpcUrl: 'https://soroban-mainnet.stellar.org',
    name: 'Mainnet',
  },
};

const CURRENT_NETWORK: StellarNetwork =
  (process.env.NEXT_PUBLIC_STELLAR_NETWORK as StellarNetwork) || 'testnet';

const NETWORK_CONFIG = NETWORKS[CURRENT_NETWORK];

export function getNetwork(): StellarNetwork {
  return CURRENT_NETWORK;
}

export function getNetworkName(): string {
  return NETWORK_CONFIG.name;
}

export class FreighterNotInstalledError extends Error {
  constructor() {
    super('Freighter wallet extension is not installed');
    this.name = 'FreighterNotInstalledError';
  }
}

export async function getWalletAddress(): Promise<string | null> {
  try {
    const result = await isConnected();
    if (!result.isConnected) return null;
    const addressResult = await getAddress();
    return addressResult.address;
  } catch (error) {
    if (
      error instanceof Error &&
      (error.message.includes('Freighter') ||
        error.message.includes('not installed') ||
        error.message.includes('extension'))
    ) {
      throw new FreighterNotInstalledError();
    }
    throw error;
  }
}

export async function safeSignTransaction(transaction: string): Promise<string> {
  try {
    const result = await signTransaction(transaction);
    return result.signedTxXdr;
  } catch (error) {
    if (
      error instanceof Error &&
      (error.message.includes('Freighter') || error.message.includes('not installed'))
    ) {
      throw new FreighterNotInstalledError();
    }
    throw error;
  }
}

export { signTransaction };

export const CONTRACT_ID =
  process.env.NEXT_PUBLIC_CONTRACT_ID ?? 'CBUWSKT2UGOAXK4ZREVDJV5XHSYB42PZ3CERU2ZFUTUMAZLJEHNZIECA';

export const NETWORK_PASSPHRASE = NETWORK_CONFIG.passphrase;

export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL ?? NETWORK_CONFIG.rpcUrl;

/**
 * Stub: call register_product on the Soroban contract.
 * Returns a simulated transaction hash.
 */
export async function registerProduct(
  productId: string,
  name: string,
  origin: string,
  description: string,
  callerAddress: string,
): Promise<string> {
  console.log('registerProduct', { productId, name, origin, description, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1200));
  return `mock_tx_${Date.now()}`;
}

/**
 * Stub: call list_products on the Soroban contract (paginated).
 */
export async function listProducts(
  page = 0,
  pageSize = 20,
): Promise<{ products: import('../types').Product[]; total: number }> {
  console.log('listProducts', { page, pageSize });
  await new Promise((r) => setTimeout(r, 800));
  return { products: [], total: 0 };
}

/**
 * Stub: call transfer_ownership on the Soroban contract.
 * Replace body with real StellarSdk contract invocation.
 */
export async function transferOwnership(
  productId: string,
  newOwner: string,
  callerAddress: string,
): Promise<void> {
  console.log('transferOwnership', { productId, newOwner, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000)); // simulate network delay
}

/**
 * Stub: call add_authorized_actor on the Soroban contract.
 * Replace body with real StellarSdk contract invocation.
 */
export async function addAuthorizedActor(
  productId: string,
  actor: string,
  callerAddress: string,
): Promise<void> {
  console.log('addAuthorizedActor', { productId, actor, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000)); // simulate network delay
}

/**
 * Stub: call remove_authorized_actor on the Soroban contract.
 * Replace body with real StellarSdk contract invocation.
 */
export async function removeAuthorizedActor(
  productId: string,
  actor: string,
  callerAddress: string,
): Promise<void> {
  console.log('removeAuthorizedActor', { productId, actor, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000)); // simulate network delay
}

/**
 * Stub: call delegate_actor_authority on the Soroban contract.
 */
export async function delegateActorAuthority(
  productId: string,
  delegatee: string,
  expiresAt: number,
  callerAddress: string,
): Promise<void> {
  console.log('delegateActorAuthority', { productId, delegatee, expiresAt, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000));
}

/**
 * Stub: call revoke_delegate on the Soroban contract.
 */
export async function revokeDelegate(
  productId: string,
  delegationId: number,
  callerAddress: string,
): Promise<void> {
  console.log('revokeDelegate', { productId, delegationId, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000));
}

/**
 * Stub: call get_active_delegations on the Soroban contract.
 */
export async function getActiveDelegations(
  productId: string,
): Promise<import('../types').Delegation[]> {
  console.log('getActiveDelegations', { productId });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 500));
  return [];
}

// ── Assembly relationship stubs ───────────────────────────────────────────────

/**
 * Stub: call register_assembly on the Soroban contract.
 * Registers a parent-child product assembly relationship on-chain.
 */
export async function registerAssembly(
  parentId: string,
  componentIds: string[],
  description: string,
  callerAddress: string,
): Promise<string> {
  console.log('registerAssembly', { parentId, componentIds, description, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1200));
  return `mock_tx_assembly_${Date.now()}`;
}

/**
 * Stub: call get_assembly on the Soroban contract.
 * Returns the assembly record for a parent product, or null if none exists.
 */
export async function getAssembly(
  parentId: string,
): Promise<import('../types').ProductAssembly | null> {
  console.log('getAssembly', { parentId });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 500));
  return null;
}

/**
 * Stub: call get_parents_of_component on the Soroban contract.
 * Returns all parent product IDs that reference the given component.
 */
export async function getParentsOfComponent(
  componentId: string,
  candidateParentIds: string[],
): Promise<string[]> {
  console.log('getParentsOfComponent', { componentId, candidateParentIds });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 500));
  return [];
}

// ── Warranty stubs ────────────────────────────────────────────────────────────

/**
 * Stub: call register_warranty on the Soroban contract.
 * Registers warranty metadata for a product on-chain.
 */
export async function registerWarranty(
  productId: string,
  durationSeconds: number,
  terms: string,
  termsRef: string,
  callerAddress: string,
): Promise<string> {
  console.log('registerWarranty', { productId, durationSeconds, terms, termsRef, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1200));
  return `mock_tx_warranty_${Date.now()}`;
}

/**
 * Stub: call get_warranty on the Soroban contract.
 * Returns warranty metadata for a product, or null if none exists.
 */
export async function getWarranty(
  productId: string,
): Promise<import('../types').WarrantyInfo | null> {
  console.log('getWarranty', { productId });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 500));
  return null;
}

/**
 * Stub: call void_warranty on the Soroban contract.
 * Voids the warranty for a product (owner-only).
 */
export async function voidWarranty(
  productId: string,
  callerAddress: string,
): Promise<string> {
  console.log('voidWarranty', { productId, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000));
  return `mock_tx_void_warranty_${Date.now()}`;
}

/**
 * Stub: call is_warranty_active on the Soroban contract.
 * Returns true if the product has an active, non-voided warranty.
 */
export async function isWarrantyActive(productId: string): Promise<boolean> {
  console.log('isWarrantyActive', { productId });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 300));
  return false;
}

/**
 * Stub: call file_warranty_claim on the Soroban contract.
 * Files a warranty claim against a product.
 */
export async function fileWarrantyClaim(
  productId: string,
  claimId: string,
  description: string,
  proofRef: string,
  callerAddress: string,
): Promise<string> {
  console.log('fileWarrantyClaim', { productId, claimId, description, proofRef, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1200));
  return `mock_tx_claim_${Date.now()}`;
}

/**
 * Stub: call list_warranty_claims on the Soroban contract.
 * Returns all warranty claims for a product.
 */
export async function listWarrantyClaims(
  productId: string,
): Promise<import('../types').WarrantyClaim[]> {
  console.log('listWarrantyClaims', { productId });
  // TODO: read from Soroban contract
  await new Promise((r) => setTimeout(r, 500));
  return [];
}

/**
 * Stub: call update_claim_status on the Soroban contract.
 * Updates the status of a warranty claim (owner-only).
 */
export async function updateClaimStatus(
  productId: string,
  claimId: string,
  newStatus: import('../types').ClaimStatus,
  callerAddress: string,
): Promise<string> {
  console.log('updateClaimStatus', { productId, claimId, newStatus, callerAddress });
  // TODO: build + sign + submit Soroban transaction
  await new Promise((r) => setTimeout(r, 1000));
  return `mock_tx_claim_status_${Date.now()}`;
}
