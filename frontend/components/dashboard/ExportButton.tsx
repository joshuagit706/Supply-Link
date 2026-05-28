"use client";

import { useState } from "react";
import { Download, Loader2 } from "lucide-react";
import type { Product, TrackingEvent } from "@/lib/types";
import { buildAuditExport, downloadJson, downloadCsv } from "@/lib/utils/export";

interface ExportButtonProps {
  product: Product;
  events: TrackingEvent[];
  format?: "json" | "csv";
}

export function ExportButton({ product, events, format = "json" }: ExportButtonProps) {
  const [loading, setLoading] = useState(false);

  async function handleExport() {
    setLoading(true);
    try {
      const filename = `audit-${product.id}-${Date.now()}`;
      if (format === "csv") {
        downloadCsv(events, `${filename}.csv`);
      } else {
        const report = await buildAuditExport(product, events);
        downloadJson(report, `${filename}.json`);
      }
    } finally {
      setLoading(false);
    }
import { Download } from "lucide-react";
import { exportToCSV, downloadCSV } from "@/lib/utils/export";
import type { TrackingEvent } from "@/lib/types";

interface ExportButtonProps {
  events: TrackingEvent[];
}

export function ExportButton({ events }: ExportButtonProps) {
  function handleExport() {
    const csv = exportToCSV(events);
    if (!csv) return;
    const date = new Date().toISOString().slice(0, 10);
    downloadCSV(csv, `supply-link-events-${date}.csv`);
  }

  return (
    <button
      onClick={handleExport}
      disabled={loading}
      aria-label={`Export audit report as ${format.toUpperCase()}`}
      className="inline-flex items-center gap-2 px-3 py-2 text-sm rounded-md border border-[var(--card-border)] bg-[var(--card)] text-[var(--foreground)] hover:bg-[var(--muted-bg)] disabled:opacity-40 transition-colors"
    >
      {loading ? (
        <Loader2 size={14} className="animate-spin" />
      ) : (
        <Download size={14} />
      )}
      Export {format.toUpperCase()}
      disabled={events.length === 0}
      className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md border border-[var(--card-border)] bg-[var(--background)] text-[var(--foreground)] hover:bg-[var(--muted-bg)] disabled:opacity-40 transition-colors"
    >
      <Download size={13} />
      Export CSV
    </button>
  );
}
