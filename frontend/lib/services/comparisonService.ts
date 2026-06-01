import type { Product, TrackingEvent } from '@/lib/types';

export interface ProvenanceMetrics {
  eventCount: number;
  actorCount: number;
  locationCount: number;
  trustScore: number;
  avgEventInterval: number;
  completenessScore: number;
}

export interface ProductComparison {
  productId: string;
  name: string;
  metrics: ProvenanceMetrics;
  commonActors: string[];
  commonLocations: string[];
}

export interface ComparisonResult {
  products: ProductComparison[];
  networkTrustSignals: {
    sharedActors: Map<string, number>;
    sharedLocations: Map<string, number>;
    trustPathStrength: number;
  };
}

export function calculateProvenanceMetrics(
  product: Product,
  events: TrackingEvent[],
): ProvenanceMetrics {
  const productEvents = events.filter((e) => e.productId === product.id);

  const actors = new Set(productEvents.map((e) => e.actor));
  const locations = new Set(productEvents.map((e) => e.location));

  let avgEventInterval = 0;
  if (productEvents.length > 1) {
    const sortedEvents = [...productEvents].sort((a, b) => a.timestamp - b.timestamp);
    const intervals = [];
    for (let i = 1; i < sortedEvents.length; i++) {
      intervals.push(sortedEvents[i].timestamp - sortedEvents[i - 1].timestamp);
    }
    avgEventInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length;
  }

  // Trust score: based on event count, actor diversity, and consistency
  const trustScore = Math.min(
    100,
    (productEvents.length * 10 + actors.size * 5 + (avgEventInterval > 0 ? 20 : 0)) / 3,
  );

  // Completeness: how many required fields are filled
  const completenessScore = Math.min(
    100,
    (productEvents.length > 0 ? 25 : 0) +
      (actors.size > 0 ? 25 : 0) +
      (locations.size > 0 ? 25 : 0) +
      (product.ownershipHistory && product.ownershipHistory.length > 0 ? 25 : 0),
  );

  return {
    eventCount: productEvents.length,
    actorCount: actors.size,
    locationCount: locations.size,
    trustScore,
    avgEventInterval,
    completenessScore,
  };
}

export function compareProducts(products: Product[], events: TrackingEvent[]): ComparisonResult {
  const comparisons: ProductComparison[] = products.map((product) => {
    const metrics = calculateProvenanceMetrics(product, events);
    const productEvents = events.filter((e) => e.productId === product.id);
    const commonActors = [...new Set(productEvents.map((e) => e.actor))];
    const commonLocations = [...new Set(productEvents.map((e) => e.location))];

    return {
      productId: product.id,
      name: product.name,
      metrics,
      commonActors,
      commonLocations,
    };
  });

  // Calculate network trust signals
  const sharedActors = new Map<string, number>();
  const sharedLocations = new Map<string, number>();

  comparisons.forEach((comp) => {
    comp.commonActors.forEach((actor) => {
      sharedActors.set(actor, (sharedActors.get(actor) || 0) + 1);
    });
    comp.commonLocations.forEach((location) => {
      sharedLocations.set(location, (sharedLocations.get(location) || 0) + 1);
    });
  });

  // Trust path strength: how interconnected the products are
  const trustPathStrength =
    (sharedActors.size * 10 + sharedLocations.size * 5) / (products.length * 15);

  return {
    products: comparisons,
    networkTrustSignals: {
      sharedActors,
      sharedLocations,
      trustPathStrength: Math.min(100, trustPathStrength * 100),
    },
  };
}
