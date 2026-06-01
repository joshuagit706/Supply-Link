'use client';

import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import {
  generateProvenanceStory,
  formatEventDate,
  formatEventTime,
  getEventColor,
  getTimeElapsed,
} from '@/lib/services/provenanceStory';
import type { TrackingEvent } from '@/lib/types';

interface ProvenanceTimelineProps {
  events: TrackingEvent[];
  productName?: string;
}

export function ProvenanceTimeline({ events, productName = 'Product' }: ProvenanceTimelineProps) {
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);
  const story = generateProvenanceStory(events);

  if (story.length === 0) {
    return (
      <div className="rounded-lg bg-gray-50 p-8 text-center">
        <p className="text-gray-600">No tracking events recorded yet.</p>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <h2 className="text-lg font-semibold text-gray-900 mb-6">Product Journey: {productName}</h2>

      <div className="relative">
        {/* Timeline line */}
        <div className="absolute left-6 top-0 bottom-0 w-1 bg-gradient-to-b from-green-400 via-blue-400 to-orange-400" />

        {/* Timeline events */}
        <div className="space-y-4">
          {story.map((segment, index) => (
            <div key={index} className="relative pl-20">
              {/* Timeline dot */}
              <div
                className={`absolute left-0 w-12 h-12 rounded-full flex items-center justify-center border-4 border-white shadow-md ${getEventColor(segment.eventType)}`}
              >
                <span className="text-lg font-bold">{index + 1}</span>
              </div>

              {/* Event card */}
              <div
                className={`rounded-lg border-2 transition-all cursor-pointer ${
                  expandedIndex === index
                    ? 'border-blue-400 bg-blue-50 shadow-lg'
                    : 'border-gray-200 bg-white hover:border-gray-300'
                }`}
                onClick={() => setExpandedIndex(expandedIndex === index ? null : index)}
              >
                <div className="p-4">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <h3 className="font-semibold text-gray-900">{segment.title}</h3>
                      <p className="text-sm text-gray-600 mt-1">{segment.location}</p>
                      <p className="text-xs text-gray-500 mt-1">
                        {formatEventDate(segment.timestamp)} at {formatEventTime(segment.timestamp)}
                      </p>
                    </div>
                    <ChevronDown
                      className={`w-5 h-5 text-gray-400 transition-transform ${
                        expandedIndex === index ? 'rotate-180' : ''
                      }`}
                    />
                  </div>

                  {/* Expanded content */}
                  {expandedIndex === index && (
                    <div className="mt-4 pt-4 border-t border-gray-200 space-y-3">
                      <div>
                        <p className="text-sm font-medium text-gray-700">Story</p>
                        <p className="text-sm text-gray-600 mt-1 leading-relaxed">
                          {segment.narrative}
                        </p>
                      </div>

                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <p className="text-xs font-medium text-gray-500 uppercase">Actor</p>
                          <p className="text-sm text-gray-900 font-mono">{segment.actor}</p>
                        </div>
                        <div>
                          <p className="text-xs font-medium text-gray-500 uppercase">Event Type</p>
                          <p className="text-sm text-gray-900">{segment.eventType}</p>
                        </div>
                      </div>

                      {/* Time elapsed to next event */}
                      {index < story.length - 1 && (
                        <div className="bg-gray-100 rounded px-3 py-2">
                          <p className="text-xs text-gray-600">
                            ⏱ Time to next stage:{' '}
                            {getTimeElapsed(segment.timestamp, story[index + 1].timestamp)}
                          </p>
                        </div>
                      )}

                      {/* Metadata if available */}
                      {segment.metadata && Object.keys(segment.metadata).length > 0 && (
                        <div>
                          <p className="text-xs font-medium text-gray-500 uppercase mb-2">
                            Additional Data
                          </p>
                          <div className="bg-gray-100 rounded p-2 text-xs font-mono text-gray-700 max-h-32 overflow-auto">
                            {JSON.stringify(segment.metadata, null, 2)}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Summary footer */}
      <div className="mt-8 pt-6 border-t border-gray-200">
        <div className="grid grid-cols-3 gap-4 text-center">
          <div>
            <p className="text-2xl font-bold text-gray-900">{story.length}</p>
            <p className="text-sm text-gray-600">Events Recorded</p>
          </div>
          <div>
            <p className="text-2xl font-bold text-gray-900">
              {getTimeElapsed(story[0].timestamp, story[story.length - 1].timestamp)}
            </p>
            <p className="text-sm text-gray-600">Total Duration</p>
          </div>
          <div>
            <p className="text-2xl font-bold text-gray-900">
              {new Set(story.map((s) => s.actor)).size}
            </p>
            <p className="text-sm text-gray-600">Participants</p>
          </div>
        </div>
      </div>
    </div>
  );
}
