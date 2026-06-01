import type { Product, TrackingEvent } from '@/lib/types';

export interface TimelineExportData {
  productId: string;
  productName: string;
  origin: string;
  owner: string;
  registeredAt: number;
  events: Array<{
    timestamp: number;
    eventType: string;
    location: string;
    actor: string;
    metadata: string;
  }>;
  exportedAt: number;
  exportFormat: 'csv' | 'json';
}

export function generateTimelineExport(
  product: Product,
  events: TrackingEvent[],
  format: 'csv' | 'json' = 'json',
): TimelineExportData {
  const productEvents = events
    .filter((e) => e.productId === product.id)
    .sort((a, b) => a.timestamp - b.timestamp);

  return {
    productId: product.id,
    productName: product.name,
    origin: product.origin,
    owner: product.owner,
    registeredAt: product.timestamp,
    events: productEvents.map((e) => ({
      timestamp: e.timestamp,
      eventType: e.eventType,
      location: e.location,
      actor: e.actor,
      metadata: e.metadata,
    })),
    exportedAt: Date.now(),
    exportFormat: format,
  };
}

export function exportToJSON(data: TimelineExportData): string {
  return JSON.stringify(data, null, 2);
}

export function exportToCSV(data: TimelineExportData): string {
  const headers = [
    'Product ID',
    'Product Name',
    'Origin',
    'Owner',
    'Registered At',
    'Event Timestamp',
    'Event Type',
    'Location',
    'Actor',
    'Metadata',
  ];

  const rows: string[] = [];

  // Add header row
  rows.push(headers.map((h) => `"${h}"`).join(','));

  // Add data rows
  if (data.events.length === 0) {
    rows.push(
      [
        `"${data.productId}"`,
        `"${data.productName}"`,
        `"${data.origin}"`,
        `"${data.owner}"`,
        data.registeredAt,
        '',
        '',
        '',
        '',
        '',
      ].join(','),
    );
  } else {
    data.events.forEach((event) => {
      rows.push(
        [
          `"${data.productId}"`,
          `"${data.productName}"`,
          `"${data.origin}"`,
          `"${data.owner}"`,
          data.registeredAt,
          event.timestamp,
          `"${event.eventType}"`,
          `"${event.location}"`,
          `"${event.actor}"`,
          `"${event.metadata.replace(/"/g, '""')}"`,
        ].join(','),
      );
    });
  }

  return rows.join('\n');
}

export function generateBatchExport(
  products: Product[],
  events: TrackingEvent[],
  format: 'csv' | 'json' = 'json',
): string {
  const exports = products.map((p) => generateTimelineExport(p, events, format));

  if (format === 'json') {
    return JSON.stringify(
      {
        exportedAt: Date.now(),
        productCount: exports.length,
        exports,
      },
      null,
      2,
    );
  } else {
    // For CSV batch export, combine all rows
    const allRows: string[] = [];
    const headers = [
      'Product ID',
      'Product Name',
      'Origin',
      'Owner',
      'Registered At',
      'Event Timestamp',
      'Event Type',
      'Location',
      'Actor',
      'Metadata',
    ];

    allRows.push(headers.map((h) => `"${h}"`).join(','));

    exports.forEach((exp) => {
      if (exp.events.length === 0) {
        allRows.push(
          [
            `"${exp.productId}"`,
            `"${exp.productName}"`,
            `"${exp.origin}"`,
            `"${exp.owner}"`,
            exp.registeredAt,
            '',
            '',
            '',
            '',
            '',
          ].join(','),
        );
      } else {
        exp.events.forEach((event) => {
          allRows.push(
            [
              `"${exp.productId}"`,
              `"${exp.productName}"`,
              `"${exp.origin}"`,
              `"${exp.owner}"`,
              exp.registeredAt,
              event.timestamp,
              `"${event.eventType}"`,
              `"${event.location}"`,
              `"${event.actor}"`,
              `"${event.metadata.replace(/"/g, '""')}"`,
            ].join(','),
          );
        });
      }
    });

    return allRows.join('\n');
  }
}
