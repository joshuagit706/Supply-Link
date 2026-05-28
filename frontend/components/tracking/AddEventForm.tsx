'use client';

import { useState, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { AlertTriangle, WifiOff } from 'lucide-react';
import { Button, Input, Select, SelectItem, FileUpload } from '@/components/ui';
import { useToast } from '@/lib/hooks/useToast';
import { EventType } from '@/lib/types';
import { EVENT_TYPE_CONFIG } from '@/lib/eventTypeConfig';
import { productIdSchema, metadataSchema } from '@/lib/validators';
import { useOfflineDraft } from '@/lib/hooks/useOfflineDraft';
import { offlineQueue } from '@/lib/offlineQueue';

const schema = z.object({
  productId: productIdSchema,
  location: z.string().min(1, 'Location is required'),
  eventType: z.enum(['HARVEST', 'PROCESSING', 'SHIPPING', 'RETAIL']),
  metadata: metadataSchema,
});

type FormValues = z.infer<typeof schema>;

interface AddEventFormProps {
  productId?: string;
  onSuccess?: () => void;
}

export function AddEventForm({ productId: initialProductId, onSuccess }: AddEventFormProps) {
  const toast = useToast();
  const [pending, setPending] = useState(false);
  const [attachmentUrl, setAttachmentUrl] = useState<string | null>(null);
  const [complianceError, setComplianceError] = useState<string | null>(null);
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== 'undefined' ? navigator.onLine : true,
  );

  const draftKey = `add-event-draft${initialProductId ? `-${initialProductId}` : ''}`;
  const { draft, saveDraft, clearDraft } = useOfflineDraft<FormValues>(draftKey);

  useEffect(() => {
    function onOnline() {
      setIsOnline(true);
    }
    function onOffline() {
      setIsOnline(false);
    }
    window.addEventListener('online', onOnline);
    window.addEventListener('offline', onOffline);
    return () => {
      window.removeEventListener('online', onOnline);
      window.removeEventListener('offline', onOffline);
    };
  }, []);

  const {
    register,
    handleSubmit,
    watch,
    reset,
    setValue,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: draft ?? {
      productId: initialProductId || '',
      location: '',
      eventType: 'HARVEST',
      metadata: '{}',
    },
  });

  const formValues = watch();

  useEffect(() => {
    saveDraft(formValues);
  }, [JSON.stringify(formValues)]);

  const eventType = watch('eventType');

  async function onSubmit(values: FormValues) {
    setComplianceError(null);

    let finalMetadata = values.metadata;
    if (attachmentUrl) {
      const parsed = JSON.parse(values.metadata || '{}');
      parsed.attachmentUrl = attachmentUrl;
      finalMetadata = JSON.stringify(parsed);
    }

    if (!isOnline) {
      offlineQueue.enqueue({ type: 'add_event', payload: { ...values, metadata: finalMetadata } });
      toast.success('Saved offline', 'Event queued and will sync when connectivity returns.');
      clearDraft();
      reset();
      setAttachmentUrl(null);
      onSuccess?.();
      return;
    }

    setPending(true);
    const toastId = toast.loading('Adding tracking event…');

    try {
      // TODO: call add_tracking_event via Soroban client with finalMetadata
      await new Promise((r) => setTimeout(r, 1200));
      const txHash = `mock_tx_${Date.now()}`;

      toast.dismiss(toastId);
      toast.success('Event added successfully', txHash);
      clearDraft();
      reset();
      setAttachmentUrl(null);
      onSuccess?.();
    } catch (err) {
      toast.dismiss(toastId);
      const message = err instanceof Error ? err.message : 'Unknown error';
      if (message.toLowerCase().includes('compliance') || message.includes('ComplianceViolation')) {
        setComplianceError(
          'This event was rejected by the product compliance policy. ' +
            'Ensure all required preceding stages have been recorded and time limits are respected.',
        );
      } else {
        toast.error('Failed to add event', message);
      }
    } finally {
      setPending(false);
    }
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col gap-4">
      {!isOnline && (
        <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-600 text-xs">
          <WifiOff size={13} />
          You are offline. The event will be queued and submitted when connectivity returns.
        </div>
      )}

      {complianceError && (
        <div className="flex items-start gap-2 px-3 py-2.5 rounded-lg bg-red-500/10 border border-red-500/20 text-red-600 text-xs">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <span>{complianceError}</span>
        </div>
      )}

      {/* Product ID */}
      <div className="flex flex-col gap-1">
        <label className="text-sm font-medium">Product ID</label>
        <Input
          {...register('productId')}
          placeholder="Enter product ID"
          disabled={!!initialProductId}
        />
        {errors.productId && <p className="text-xs text-red-500">{errors.productId.message}</p>}
      </div>

      {/* Location */}
      <div className="flex flex-col gap-1">
        <label className="text-sm font-medium">Location</label>
        <Input {...register('location')} placeholder="e.g. Warehouse A, Port of Shanghai" />
        {errors.location && <p className="text-xs text-red-500">{errors.location.message}</p>}
      </div>

      {/* Event Type */}
      <div className="flex flex-col gap-1">
        <label className="text-sm font-medium">Event Type</label>
        <Select value={eventType} onValueChange={(val) => setValue('eventType', val as EventType)}>
          {(['HARVEST', 'PROCESSING', 'SHIPPING', 'RETAIL'] as EventType[]).map((t) => {
            const cfg = EVENT_TYPE_CONFIG[t];
            const Icon = cfg.icon;
            return (
              <SelectItem key={t} value={t}>
                <span
                  className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${cfg.badgeClass}`}
                >
                  <Icon size={11} />
                  {cfg.label}
                </span>
              </SelectItem>
            );
          })}
        </Select>
        {errors.eventType && <p className="text-xs text-red-500">{errors.eventType.message}</p>}
      </div>

      {/* Metadata */}
      <div className="flex flex-col gap-1">
        <label className="text-sm font-medium">
          Metadata <span className="text-[var(--muted)] font-normal">(JSON)</span>
        </label>
        <textarea
          {...register('metadata')}
          rows={4}
          placeholder='{"temperature": 25, "humidity": 60}'
          className="px-3 py-2 rounded-lg border border-[var(--card-border)] bg-[var(--card)] text-sm font-mono focus:outline-none focus:ring-2 focus:ring-violet-500 resize-none"
        />
        {errors.metadata && <p className="text-xs text-red-500">{errors.metadata.message}</p>}
      </div>

      {/* File Attachment */}
      <FileUpload
        onUpload={(url) => setAttachmentUrl(url)}
        onClear={() => setAttachmentUrl(null)}
      />

      <Button type="submit" disabled={pending}>
        {pending ? 'Adding…' : isOnline ? 'Add Event' : 'Queue Offline'}
      </Button>
    </form>
  );
}
