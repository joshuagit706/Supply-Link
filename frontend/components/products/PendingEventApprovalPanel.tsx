"use client";

import { useState } from "react";
import { CheckCircle, XCircle, Clock, Loader2 } from "lucide-react";
import type { PendingEvent } from "@/lib/types";
import { useStore } from "@/lib/state/store";

interface PendingEventApprovalPanelProps {
  productId: string;
  pendingEvents: PendingEvent[];
  onApprove?: (productId: string, submitter: string) => Promise<void>;
  onReject?: (productId: string, submitter: string) => Promise<void>;
}

function isExpired(event: PendingEvent): boolean {
  return Date.now() / 1000 > event.expiresAt;
}

function approvalProgress(event: PendingEvent): string {
  return `${event.approvals.length} / ${event.requiredApprovers.length}`;
}

export function PendingEventApprovalPanel({
  productId,
  pendingEvents,
  onApprove,
  onReject,
}: PendingEventApprovalPanelProps) {
  const walletAddress = useStore((s) => s.walletAddress);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState("");

  async function handleApprove(event: PendingEvent) {
    if (!walletAddress) { setError("Connect your wallet first."); return; }
    setPending(event.submitter);
    setError("");
    try {
      await onApprove?.(productId, event.submitter);
    } catch {
      setError("Transaction failed. Please try again.");
    } finally {
      setPending(null);
    }
  }

  async function handleReject(event: PendingEvent) {
    if (!walletAddress) { setError("Connect your wallet first."); return; }
    setPending(event.submitter);
    setError("");
    try {
      await onReject?.(productId, event.submitter);
    } catch {
      setError("Transaction failed. Please try again.");
    } finally {
      setPending(null);
    }
  }

  if (pendingEvents.length === 0) {
    return (
      <p className="text-sm text-[var(--muted)] py-4 text-center">
        No pending events awaiting approval.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {error && <p className="text-xs text-red-500">{error}</p>}
      {pendingEvents.map((event) => {
        const expired = isExpired(event);
        const isApprover = walletAddress
          ? event.requiredApprovers.includes(walletAddress)
          : false;
        const alreadyApproved = walletAddress
          ? event.approvals.includes(walletAddress)
          : false;
        const isPending = pending === event.submitter;

        return (
          <div
            key={event.submitter}
            className={`border border-[var(--card-border)] rounded-xl p-4 flex flex-col gap-3 ${
              expired || event.rejected ? "opacity-50" : ""
            }`}
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex flex-col gap-1">
                <span className="text-xs font-semibold text-[var(--foreground)]">
                  {event.eventType} — {event.location}
                </span>
                <span className="text-xs text-[var(--muted)] font-mono">
                  Submitted by {event.submitter.slice(0, 8)}…{event.submitter.slice(-6)}
                </span>
                <span className="text-xs text-[var(--muted)]">
                  {new Date(event.submittedAt * 1000).toLocaleString()}
                </span>
              </div>
              <div className="flex flex-col items-end gap-1">
                {event.rejected && (
                  <span className="text-xs text-red-500 font-medium">Rejected</span>
                )}
                {expired && !event.rejected && (
                  <span className="text-xs text-amber-500 font-medium flex items-center gap-1">
                    <Clock size={11} /> Expired
                  </span>
                )}
                {!expired && !event.rejected && (
                  <span className="text-xs text-[var(--muted)]">
                    Approvals: {approvalProgress(event)}
                  </span>
                )}
              </div>
            </div>

            {!expired && !event.rejected && isApprover && !alreadyApproved && (
              <div className="flex gap-2">
                <button
                  onClick={() => handleApprove(event)}
                  disabled={isPending}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-40 transition-colors"
                >
                  {isPending ? <Loader2 size={12} className="animate-spin" /> : <CheckCircle size={12} />}
                  Approve
                </button>
                <button
                  onClick={() => handleReject(event)}
                  disabled={isPending}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-40 transition-colors"
                >
                  {isPending ? <Loader2 size={12} className="animate-spin" /> : <XCircle size={12} />}
                  Reject
                </button>
              </div>
            )}
            {alreadyApproved && (
              <span className="text-xs text-green-600 flex items-center gap-1">
                <CheckCircle size={11} /> You approved this event
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
