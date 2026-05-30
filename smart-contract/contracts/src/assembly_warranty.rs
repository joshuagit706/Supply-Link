/// assembly_warranty.rs
///
/// Product assembly relationships (parent-child) and warranty tracking.
///
/// # Assembly
/// A parent product can reference one or more component products. The
/// relationship is stored as a [`ProductAssembly`] record keyed by the parent
/// product ID. Components carry their own provenance chains; the assembly
/// record aggregates them under a single parent.
///
/// # Warranty
/// A product can have warranty metadata stored on-chain. Warranty claims are
/// linked to the product's provenance and stored as [`WarrantyClaim`] records.

#![allow(unused_imports)]
use soroban_sdk::{contracttype, Address, Env, String, Vec, Symbol};

use crate::{DataKey, Error, Product, MAX_ID_LEN, MAX_METADATA_LEN};

// ── Size limits ───────────────────────────────────────────────────────────────

/// Maximum number of component products in a single assembly.
pub const MAX_ASSEMBLY_COMPONENTS: u32 = 50;

/// Maximum length of a warranty terms string.
pub const MAX_WARRANTY_TERMS_LEN: u32 = 1024;

/// Maximum length of a claim proof reference (e.g. IPFS CID or URL).
pub const MAX_CLAIM_PROOF_LEN: u32 = 512;

// ── Data types ────────────────────────────────────────────────────────────────

/// Represents a parent-child product assembly relationship.
///
/// A parent product is assembled from one or more component products. Each
/// component carries its own provenance chain. The assembly record stores the
/// ordered list of component IDs and an optional description of the assembly
/// process.
///
/// # Storage
/// Stored under [`DataKey::Assembly`] keyed by the parent product ID.
/// Persistent storage — survives ledger expiry.
#[contracttype]
#[derive(Clone)]
pub struct ProductAssembly {
    /// ID of the parent (assembled) product.
    pub parent_id: String,
    /// Ordered list of component product IDs.
    pub component_ids: Vec<String>,
    /// Address of the actor who registered this assembly relationship.
    pub registered_by: Address,
    /// Ledger timestamp when the assembly was registered.
    pub registered_at: u64,
    /// Optional free-form description of the assembly process.
    pub description: String,
}

/// Warranty metadata for a product.
///
/// Stored on-chain alongside the product. Warranty coverage is defined by
/// `duration_seconds` from the product's registration timestamp. The
/// `terms_ref` field can hold an IPFS CID or URL pointing to the full
/// warranty document stored off-chain.
///
/// # Storage
/// Stored under [`DataKey::Warranty`] keyed by the product ID.
#[contracttype]
#[derive(Clone)]
pub struct WarrantyInfo {
    /// ID of the product this warranty covers.
    pub product_id: String,
    /// Warranty duration in seconds from product registration timestamp.
    /// 0 means no expiry (lifetime warranty).
    pub duration_seconds: u64,
    /// Address of the actor who registered the warranty (typically the owner).
    pub issuer: Address,
    /// Ledger timestamp when the warranty was registered.
    pub issued_at: u64,
    /// Short human-readable summary of warranty terms (max 1024 bytes).
    pub terms: String,
    /// Off-chain reference to the full warranty document (IPFS CID, URL, etc.).
    /// Max 512 bytes.
    pub terms_ref: String,
    /// Whether this warranty has been voided.
    pub voided: bool,
    /// Ledger timestamp when the warranty was voided (0 if not voided).
    pub voided_at: u64,
}

/// A warranty claim filed against a product.
///
/// Claims are append-only. Each claim references the product's provenance
/// (via `provenance_ref`) and optionally includes a proof document reference.
///
/// # Storage
/// Stored as a `Vec<WarrantyClaim>` under [`DataKey::WarrantyClaims`] keyed
/// by the product ID.
#[contracttype]
#[derive(Clone)]
pub struct WarrantyClaim {
    /// Stable unique identifier for this claim (caller-supplied).
    pub claim_id: String,
    /// ID of the product the claim is filed against.
    pub product_id: String,
    /// Address of the claimant.
    pub claimant: Address,
    /// Ledger timestamp when the claim was filed.
    pub filed_at: u64,
    /// Short description of the issue (stored in metadata JSON, max 4096 bytes).
    pub description: String,
    /// Off-chain proof reference (IPFS CID, URL, etc.). Max 512 bytes.
    pub proof_ref: String,
    /// Current status of the claim.
    pub status: ClaimStatus,
    /// Ledger timestamp when the claim status was last updated.
    pub updated_at: u64,
}

/// Lifecycle status of a warranty claim.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimStatus {
    /// Claim has been filed and is awaiting review.
    Pending,
    /// Claim has been approved by the product owner.
    Approved,
    /// Claim has been rejected by the product owner.
    Rejected,
    /// Claim has been resolved (repair/replacement completed).
    Resolved,
}

// ── Storage key extensions ────────────────────────────────────────────────────
// These variants are added to the existing DataKey enum in lib.rs via
// the assembly_warranty module. They are declared here for documentation
// purposes; the actual DataKey enum in lib.rs must include them.
//
// DataKey::Assembly(String)        — ProductAssembly keyed by parent_id
// DataKey::Warranty(String)        — WarrantyInfo keyed by product_id
// DataKey::WarrantyClaims(String)  — Vec<WarrantyClaim> keyed by product_id

// ── Assembly helpers ──────────────────────────────────────────────────────────

/// Register or replace the assembly relationship for a parent product.
///
/// # Authorization
/// Requires the parent product owner's auth.
///
/// # Panics
/// - `"parent product not found"` — if `parent_id` is not registered.
/// - `"component product not found: <id>"` — if any component ID is not registered.
/// - `"assembly exceeds maximum component count"` — if more than 50 components.
/// - `"description exceeds max length"` — if description > 4096 bytes.
pub fn register_assembly(
    env: &Env,
    parent_id: String,
    component_ids: Vec<String>,
    description: String,
    caller: Address,
) -> Result<ProductAssembly, Error> {
    // Validate parent exists
    let parent: Product = env
        .storage()
        .persistent()
        .get(&DataKey::Product(parent_id.clone()))
        .ok_or(Error::ProductNotFound)?;

    // Only owner may register assembly
    if parent.owner != caller {
        return Err(Error::NotAuthorized);
    }
    caller.require_auth();

    // Validate component count
    if component_ids.len() > MAX_ASSEMBLY_COMPONENTS {
        panic!("assembly exceeds maximum component count");
    }

    // Validate description length
    if description.len() > MAX_METADATA_LEN {
        panic!("description exceeds max length");
    }

    // Validate all components exist
    for i in 0..component_ids.len() {
        let cid = component_ids.get(i).unwrap();
        if !env.storage().persistent().has(&DataKey::Product(cid.clone())) {
            panic!("component product not found");
        }
    }

    let assembly = ProductAssembly {
        parent_id: parent_id.clone(),
        component_ids,
        registered_by: caller,
        registered_at: env.ledger().timestamp(),
        description,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Assembly(parent_id.clone()), &assembly);

    env.events().publish(
        (Symbol::new(env, "assembly_registered"), parent_id),
        assembly.clone(),
    );

    Ok(assembly)
}

/// Retrieve the assembly record for a parent product.
///
/// Returns `None` if no assembly has been registered for this product.
pub fn get_assembly(env: &Env, parent_id: String) -> Option<ProductAssembly> {
    env.storage()
        .persistent()
        .get(&DataKey::Assembly(parent_id))
}

/// Return all parent product IDs that reference `component_id` as a component.
///
/// This is a linear scan over all assemblies stored under the provided
/// `parent_ids` hint list. In a production system this would be maintained
/// as a reverse index; here we accept a caller-supplied list of candidate
/// parent IDs to keep storage costs bounded.
pub fn get_parents_of_component(
    env: &Env,
    component_id: String,
    candidate_parent_ids: Vec<String>,
) -> Vec<String> {
    let mut parents = Vec::new(env);
    for i in 0..candidate_parent_ids.len() {
        let pid = candidate_parent_ids.get(i).unwrap();
        if let Some(assembly) = env
            .storage()
            .persistent()
            .get::<DataKey, ProductAssembly>(&DataKey::Assembly(pid.clone()))
        {
            if assembly.component_ids.contains(&component_id) {
                parents.push_back(pid);
            }
        }
    }
    parents
}

// ── Warranty helpers ──────────────────────────────────────────────────────────

/// Register warranty metadata for a product.
///
/// # Authorization
/// Requires the product owner's auth.
///
/// # Panics
/// - `"product not found"` — if `product_id` is not registered.
/// - `"terms exceeds max length"` — if terms > 1024 bytes.
/// - `"terms_ref exceeds max length"` — if terms_ref > 512 bytes.
pub fn register_warranty(
    env: &Env,
    product_id: String,
    duration_seconds: u64,
    terms: String,
    terms_ref: String,
    caller: Address,
) -> Result<WarrantyInfo, Error> {
    let product: Product = env
        .storage()
        .persistent()
        .get(&DataKey::Product(product_id.clone()))
        .ok_or(Error::ProductNotFound)?;

    if product.owner != caller {
        return Err(Error::NotAuthorized);
    }
    caller.require_auth();

    if terms.len() > MAX_WARRANTY_TERMS_LEN {
        panic!("terms exceeds max length");
    }
    if terms_ref.len() > MAX_CLAIM_PROOF_LEN {
        panic!("terms_ref exceeds max length");
    }

    let warranty = WarrantyInfo {
        product_id: product_id.clone(),
        duration_seconds,
        issuer: caller,
        issued_at: env.ledger().timestamp(),
        terms,
        terms_ref,
        voided: false,
        voided_at: 0,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Warranty(product_id.clone()), &warranty);

    env.events().publish(
        (Symbol::new(env, "warranty_registered"), product_id),
        warranty.clone(),
    );

    Ok(warranty)
}

/// Retrieve warranty metadata for a product.
///
/// Returns `None` if no warranty has been registered.
pub fn get_warranty(env: &Env, product_id: String) -> Option<WarrantyInfo> {
    env.storage()
        .persistent()
        .get(&DataKey::Warranty(product_id))
}

/// Void a warranty. Owner-only.
///
/// # Panics
/// - `"product not found"` — if `product_id` is not registered.
/// - `"no warranty registered"` — if no warranty exists for this product.
pub fn void_warranty(
    env: &Env,
    product_id: String,
    caller: Address,
) -> Result<WarrantyInfo, Error> {
    let product: Product = env
        .storage()
        .persistent()
        .get(&DataKey::Product(product_id.clone()))
        .ok_or(Error::ProductNotFound)?;

    if product.owner != caller {
        return Err(Error::NotAuthorized);
    }
    caller.require_auth();

    let mut warranty: WarrantyInfo = env
        .storage()
        .persistent()
        .get(&DataKey::Warranty(product_id.clone()))
        .ok_or_else(|| panic!("no warranty registered"))?;

    warranty.voided = true;
    warranty.voided_at = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&DataKey::Warranty(product_id.clone()), &warranty);

    env.events().publish(
        (Symbol::new(env, "warranty_voided"), product_id),
        warranty.clone(),
    );

    Ok(warranty)
}

/// Check whether a product's warranty is currently active.
///
/// Returns `true` if:
/// - A warranty exists for the product, AND
/// - The warranty has not been voided, AND
/// - Either `duration_seconds == 0` (lifetime) OR the current ledger time
///   is within `product.timestamp + duration_seconds`.
pub fn is_warranty_active(env: &Env, product_id: String) -> bool {
    let product: Product = match env
        .storage()
        .persistent()
        .get(&DataKey::Product(product_id.clone()))
    {
        Some(p) => p,
        None => return false,
    };

    let warranty: WarrantyInfo = match env
        .storage()
        .persistent()
        .get(&DataKey::Warranty(product_id))
    {
        Some(w) => w,
        None => return false,
    };

    if warranty.voided {
        return false;
    }

    if warranty.duration_seconds == 0 {
        return true; // lifetime warranty
    }

    let expiry = product.timestamp + warranty.duration_seconds;
    env.ledger().timestamp() <= expiry
}

// ── Warranty claim helpers ────────────────────────────────────────────────────

/// File a warranty claim against a product.
///
/// # Authorization
/// Any address may file a claim (the claimant self-authorizes).
///
/// # Panics
/// - `"product not found"` — if `product_id` is not registered.
/// - `"no warranty registered"` — if no warranty exists.
/// - `"warranty is voided"` — if the warranty has been voided.
/// - `"claim_id exceeds max length"` — if claim_id > 128 bytes.
/// - `"description exceeds max length"` — if description > 4096 bytes.
/// - `"proof_ref exceeds max length"` — if proof_ref > 512 bytes.
pub fn file_warranty_claim(
    env: &Env,
    product_id: String,
    claim_id: String,
    description: String,
    proof_ref: String,
    claimant: Address,
) -> Result<WarrantyClaim, Error> {
    // Validate product exists
    if !env
        .storage()
        .persistent()
        .has(&DataKey::Product(product_id.clone()))
    {
        return Err(Error::ProductNotFound);
    }

    // Validate warranty exists and is not voided
    let warranty: WarrantyInfo = env
        .storage()
        .persistent()
        .get(&DataKey::Warranty(product_id.clone()))
        .ok_or_else(|| panic!("no warranty registered"))?;

    if warranty.voided {
        panic!("warranty is voided");
    }

    claimant.require_auth();

    if claim_id.len() > MAX_ID_LEN {
        panic!("claim_id exceeds max length");
    }
    if description.len() > MAX_METADATA_LEN {
        panic!("description exceeds max length");
    }
    if proof_ref.len() > MAX_CLAIM_PROOF_LEN {
        panic!("proof_ref exceeds max length");
    }

    let claim = WarrantyClaim {
        claim_id: claim_id.clone(),
        product_id: product_id.clone(),
        claimant,
        filed_at: env.ledger().timestamp(),
        description,
        proof_ref,
        status: ClaimStatus::Pending,
        updated_at: env.ledger().timestamp(),
    };

    let mut claims: Vec<WarrantyClaim> = env
        .storage()
        .persistent()
        .get(&DataKey::WarrantyClaims(product_id.clone()))
        .unwrap_or_else(|| Vec::new(env));

    claims.push_back(claim.clone());
    env.storage()
        .persistent()
        .set(&DataKey::WarrantyClaims(product_id.clone()), &claims);

    env.events().publish(
        (Symbol::new(env, "warranty_claim_filed"), product_id),
        claim.clone(),
    );

    Ok(claim)
}

/// Update the status of a warranty claim. Owner-only.
///
/// # Authorization
/// Requires the product owner's auth.
///
/// # Panics
/// - `"product not found"` — if `product_id` is not registered.
/// - `"claim not found"` — if no claim with `claim_id` exists.
pub fn update_claim_status(
    env: &Env,
    product_id: String,
    claim_id: String,
    new_status: ClaimStatus,
    caller: Address,
) -> Result<WarrantyClaim, Error> {
    let product: Product = env
        .storage()
        .persistent()
        .get(&DataKey::Product(product_id.clone()))
        .ok_or(Error::ProductNotFound)?;

    if product.owner != caller {
        return Err(Error::NotAuthorized);
    }
    caller.require_auth();

    let mut claims: Vec<WarrantyClaim> = env
        .storage()
        .persistent()
        .get(&DataKey::WarrantyClaims(product_id.clone()))
        .unwrap_or_else(|| Vec::new(env));

    let mut found_index: Option<u32> = None;
    for i in 0..claims.len() {
        if claims.get(i).unwrap().claim_id == claim_id {
            found_index = Some(i);
            break;
        }
    }

    let idx = found_index.ok_or_else(|| panic!("claim not found"))?;
    let mut claim = claims.get(idx).unwrap().clone();
    claim.status = new_status;
    claim.updated_at = env.ledger().timestamp();
    claims.set(idx, claim.clone());

    env.storage()
        .persistent()
        .set(&DataKey::WarrantyClaims(product_id.clone()), &claims);

    env.events().publish(
        (Symbol::new(env, "warranty_claim_updated"), product_id),
        claim.clone(),
    );

    Ok(claim)
}

/// Return all warranty claims for a product.
pub fn list_warranty_claims(env: &Env, product_id: String) -> Vec<WarrantyClaim> {
    env.storage()
        .persistent()
        .get(&DataKey::WarrantyClaims(product_id))
        .unwrap_or_else(|| Vec::new(env))
}
