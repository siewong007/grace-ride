import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Calendar, CarFront, Repeat } from 'lucide-react';

export interface TimelineNode {
  id: string;
  timeLabel: string;
  title: string;
  subtitle?: string;
  to?: string;
  kind: 'ride' | 'return' | 'drive' | 'event';
  state: 'active' | 'upcoming' | 'done';
  badge?: ReactNode;
  action?: ReactNode;
}

export interface DayOption {
  key: string;
  weekday: string;
  dateLabel: string;
  count: number;
}

function nodeIcon(kind: TimelineNode['kind']) {
  if (kind === 'return') return <Repeat size={15} aria-hidden="true" />;
  if (kind === 'event') return <Calendar size={15} aria-hidden="true" />;
  return <CarFront size={15} aria-hidden="true" />;
}

/** Vertical chronological rail uniting rides and events into one quiet day view. */
export function ScheduleTimeline({
  items,
  compact = false,
}: {
  items: TimelineNode[];
  compact?: boolean;
}) {
  if (items.length === 0) return null;

  return (
    <ol className={`schedule-timeline${compact ? ' schedule-timeline--compact' : ''}`}>
      {items.map((item, idx) => {
        const isLast = idx === items.length - 1;
        const inner = (
          <>
            <div className="row between wrap gap-xs">
              <span className="schedule-node-title bold">{item.title}</span>
              {item.badge}
            </div>
            {item.subtitle && <span className="muted small clamp-2">{item.subtitle}</span>}
          </>
        );

        return (
          <li key={item.id} className={`schedule-node schedule-node--${item.state}`}>
            <span className="schedule-node-time">{item.timeLabel}</span>
            <div className="schedule-node-rail" aria-hidden="true">
              <span className="schedule-node-dot" />
              {!isLast && <span className="schedule-node-line" />}
            </div>
            <div className="schedule-node-body">
              {item.to ? (
                <Link to={item.to} className="schedule-node-link">
                  {inner}
                </Link>
              ) : (
                <div className="schedule-node-static">{inner}</div>
              )}
              {item.action && <div className="schedule-node-action">{item.action}</div>}
            </div>
            <div className="schedule-node-icon" aria-hidden="true">
              {nodeIcon(item.kind)}
            </div>
          </li>
        );
      })}
    </ol>
  );
}

/** Horizontal day switcher for multi-day event schedules without card boxes or gradients. */
export function DayStrip({
  days,
  selectedDay,
  onSelect,
  allLabel,
  ariaLabel,
}: {
  days: DayOption[];
  selectedDay: string;
  onSelect: (key: string) => void;
  allLabel: string;
  ariaLabel?: string;
}) {
  return (
    <div
      className="day-strip"
      role="group"
      aria-label={ariaLabel}
      data-no-swipe-back
      data-no-swipe-next
    >
      <button
        type="button"
        className={`day-strip-btn${selectedDay === 'all' ? ' is-active' : ''}`}
        aria-pressed={selectedDay === 'all'}
        onClick={() => onSelect('all')}
      >
        <span className="day-strip-weekday">{allLabel}</span>
      </button>
      {days.map((d) => {
        const active = selectedDay === d.key;
        return (
          <button
            key={d.key}
            type="button"
            className={`day-strip-btn${active ? ' is-active' : ''}`}
            aria-pressed={active}
            onClick={() => onSelect(d.key)}
          >
            <span className="day-strip-weekday">{d.weekday}</span>
            <span className="day-strip-date">{d.dateLabel}</span>
          </button>
        );
      })}
    </div>
  );
}
