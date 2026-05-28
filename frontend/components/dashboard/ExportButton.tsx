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
    </button>
  );
}
