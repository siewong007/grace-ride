export interface PickupArea {
  id: number;
  name: string;
  active: boolean;
}

export interface Group {
  id: number;
  name: string;
  description: string;
  is_all: boolean;
  member_count: number;
  parent_id: number | null;
  is_visitor: boolean;
}

export interface GroupSummary {
  id: number;
  name: string;
  is_all: boolean;
  is_visitor?: boolean;
}

export interface Place {
  label: string;
  lat: number;
  lng: number;
}

export interface SavedPlace {
  id: number;
  label: string;
  area_id: number | null;
  area_name: string | null;
  address: string;
  gate_notes: string;
  lat?: number | null;
  lng?: number | null;
  is_default: number;
}

export interface ProxyRider {
  id: number;
  name: string;
  phone: string;
  relationship: string;
  notes: string;
  area_id?: number | null;
  pickup_address?: string;
  gate_notes?: string;
}

export type BadgeTier = 'none' | 'bronze' | 'silver' | 'gold' | 'platinum';
export type EarnedBadgeTier = Exclude<BadgeTier, 'none'>;

export interface BadgeTierStep {
  tier: EarnedBadgeTier;
  target: number;
  reached: boolean;
}

export interface Badge {
  key: string;
  category?: 'member' | 'driver' | 'traffic_lead';
  role_match?: boolean;
  char: string;
  icon?: string;
  name: string;
  rule: string;
  char_zh?: string;
  char_en?: string;
  name_zh?: string;
  name_en?: string;
  rule_zh?: string;
  rule_en?: string;
  progress: number;
  target: number;
  earned: boolean;
  tier?: BadgeTier;
  tier_level?: number;
  next_tier?: EarnedBadgeTier | null;
  next_target?: number | null;
  tiers?: BadgeTierStep[];
}

export interface MonthReportVisibleSections {
  events: boolean;
  driving: boolean;
  passenger_rides: boolean;
  costs_paid: boolean;
  reimbursements: boolean;
  organised: boolean;
  badges: boolean;
}

export interface MonthReportReimbursementsSummary {
  claim_count: number;
  pending_count: number | null;
  pending_cents: number | null;
  approved_count: number;
  approved_cents: number;
  paid_count: number;
  paid_cents: number;
  returned_count: number | null;
  rejected_count: number | null;
}

export interface MonthReportOrganisedEvents {
  events_count: number;
  tracked_events_count?: number;
  total_going: number;
  total_attended: number;
  attendance_rate_pct: number;
  settlements: {
    count: number;
    gross_cents: number;
    subsidy_cents: number;
    collected_cents: number;
    outstanding_cents: number;
  } | null;
}

export interface MonthReport {
  user: { id: number; name: string };
  month: string;
  range: string;
  type: string;
  period_start: string;
  period_end: string;
  period_end_inclusive?: string;
  rides_driven: number | null;
  rides_received: number | null;
  distance_km: number | null;
  reimbursement_cents: number | null;
  events_attended: number | null;
  by_kind: { kind: string; count: number }[];
  events: { id: number; title: string; kind: string; starts_at: string }[];
  costs_paid_cents: number | null;
  visible_sections?: MonthReportVisibleSections;
  reimbursements_summary?: MonthReportReimbursementsSummary | null;
  organised_events?: MonthReportOrganisedEvents | null;
  badges: Badge[];
}

export type ReportScopeKind = 'none' | 'own_events' | 'global';

export interface ReportCapabilities {
  overview: boolean;
  member_reports_global: boolean;
  member_badge_icon_edit: boolean;
  demand: boolean;
  demand_scope: ReportScopeKind;
  events: boolean;
  events_scope: ReportScopeKind;
  finance: boolean;
  finance_sections: {
    reimb_review: boolean;
    reimb_payouts: boolean;
    bills: boolean;
  };
  membership: boolean;
}

export interface OverviewReportData {
  period: string;
  range: string;
  from: string;
  to: string;
  comparison?: {
    prev_period: string;
    transport: {
      total_rides_delta: number;
      completed_rides_delta: number;
      fulfillment_rate_delta: number;
    } | null;
    events: {
      events_count_delta: number;
      total_attended_delta: number;
      attendance_rate_delta: number;
    } | null;
    reimb_review: {
      pending_count_delta: number;
    } | null;
    reimb_payouts: {
      paid_cents_delta: number;
    } | null;
    bills: {
      collected_cents_delta: number;
      collection_rate_delta: number;
    } | null;
    membership: {
      new_members_delta: number;
    } | null;
  } | null;
  transport: {
    scope: 'global' | 'own_events';
    total_rides: number;
    completed_rides: number;
    pending_confirmation_rides?: number;
    unfilled_rides: number;
    cancelled_rides: number;
    no_show_rides: number;
    alt_transport_rides: number;
    fulfillment_rate_pct: number;
    total_distance_km: number;
    active_drivers: number;
    unique_passengers: number;
    top_drivers: {
      user_id: number;
      name: string;
      completed_rides: number;
      distance_km: number;
      carpool_trips: number;
    }[];
  } | null;
  events: {
    scope: 'global' | 'own_events';
    events_count: number;
    tracked_events_count?: number;
    total_going: number;
    total_attended: number;
    attendance_rate_pct: number;
    top_groups: {
      group_id: number | null;
      group_name: string | null;
      going: number;
      attended: number;
    }[];
  } | null;
  reimb_review: {
    pending_count: number;
    pending_cents: number;
    returned_count: number;
    approved_count: number;
    approved_cents: number;
  } | null;
  finance: {
    reimb_payouts: {
      approved_unpaid_count: number;
      approved_unpaid_cents: number;
      paid_count: number;
      paid_cents: number;
      open_batches: number;
      paid_batches: number;
    } | null;
    bills: {
      published_count: number;
      closed_count: number;
      gross_cents: number;
      subsidy_cents: number;
      collected_cents: number;
      outstanding_cents: number;
      collection_rate_pct: number;
    } | null;
  } | null;
  membership: {
    active_members: number;
    pending_members: number;
    new_members_in_period: number;
    active_drivers_total: number;
  } | null;
  monthly_series: {
    month: string;
    rides_completed: number | null;
    events_attended: number | null;
    reimb_paid_cents: number | null;
    bills_collected_cents: number | null;
  }[];
}

export interface EventsReportData {
  scope: 'global' | 'own_events';
  period: string;
  range: string;
  from: string;
  to: string;
  events_total?: number;
  events_truncated?: boolean;
  comparison?: {
    prev_period: string;
    events_count_delta: number;
    total_going_delta: number;
    total_attended_delta: number;
    attendance_rate_delta: number;
  } | null;
  totals: {
    events_count: number;
    tracked_events_count?: number;
    cancelled_events: number;
    total_going: number;
    total_waitlist: number;
    total_attended: number;
    attendance_rate_pct: number;
    need_ride_count: number;
    offer_drive_count: number;
    self_transport_count: number;
    visitor_passes_count: number;
    plan_visitors_count: number | null;
  };
  by_group: {
    group_id: number | null;
    group_name: string | null;
    going: number;
    attended: number;
    attendance_rate_pct: number;
    need_ride: number;
    self_transport: number;
    offer_drive: number;
  }[];
  by_kind: {
    kind: string;
    events_count: number;
    going: number;
    attended: number;
    attendance_rate_pct: number;
  }[];
  events: {
    id: number;
    title: string;
    kind: string;
    starts_at: string;
    status: string;
    creator_name: string;
    going: number;
    waitlist: number;
    attended: number;
    attendance_enabled: boolean;
    attendance_rate_pct: number | null;
    need_ride: number;
    self_transport: number;
    rides_matched: number | null;
    rides_unfilled: number | null;
  }[];
}

export interface DemandReportData {
  scope?: 'global' | 'own_events';
  period: string;
  range?: string;
  from?: string;
  to?: string;
  comparison?: {
    prev_period: string;
    total_requests_delta: number;
    unfilled_delta: number;
    fulfillment_rate_delta: number;
  } | null;
  totals: {
    total_requests: number;
    fulfilled: number;
    pending_confirmation?: number;
    unfilled: number;
    completed: number;
    no_show?: number;
    alt_transport?: number;
    cancelled_total?: number;
    cancelled_unassigned: number;
    wheelchair_requests: number;
    stroller_requests?: number;
    oxygen_requests?: number;
    return_requests: number;
    proxy_rides?: number;
    plan_visitors_count?: number | null;
    plan_visitors_seats?: number | null;
    visitor_passes_count?: number;
    seats_unfilled: number;
    active_drivers: number;
    unique_passengers: number;
    fulfillment_rate_pct: number;
  };
  by_area: {
    area_id: number;
    area_name_zh: string;
    area_name_en: string;
    total: number;
    fulfilled: number;
    unfilled: number;
    cancelled_unassigned: number;
    seats_needed: number;
    seats_unfilled: number;
    fulfillment_rate_pct: number;
    registered_drivers?: number | null;
  }[];
  by_slot: {
    weekday: number;
    slot: string;
    total: number;
    fulfilled: number;
    unfilled: number;
    cancelled_unassigned: number;
    seats_needed: number;
  }[];
  by_group?: {
    group_id: number | null;
    group_name: string | null;
    total: number;
    fulfilled: number;
    unfilled: number;
    seats_needed: number;
    fulfillment_rate_pct: number;
  }[];
}

export interface FinanceReportData {
  period: string;
  range: string;
  from: string;
  to: string;
  permissions: {
    reimb_review: boolean;
    reimb_payouts: boolean;
    bills: boolean;
  };
  comparison?: {
    prev_period: string;
    reimb_review: {
      pending_count_delta: number;
      approved_cents_delta: number;
    } | null;
    reimb_payouts: {
      paid_count_delta: number;
      paid_cents_delta: number;
    } | null;
    bills: {
      collected_cents_delta: number;
      collection_rate_delta: number;
    } | null;
  } | null;
  reimbursement_review: {
    by_status: Record<
      'pending' | 'returned' | 'rejected' | 'approved' | 'paid',
      { count: number; amount_cents: number }
    >;
    overdue_pending_count?: number;
    by_kind: { kind: string; count: number; amount_cents: number }[];
    high_amount_special_count: number;
    driver_mileage_rides: {
      eligible_count: number;
      eligible_cents: number;
      excluded_count: number;
      receipt_needed_count: number;
    };
  } | null;
  reimbursement_payouts: {
    approved_unpaid_count: number;
    approved_unpaid_cents: number;
    all_time_awaiting_count: number;
    all_time_awaiting_cents: number;
    overdue_approved_count?: number;
    overdue_approved_cents?: number;
    paid_count: number;
    paid_cents: number;
    payee_verified_count: number;
    payee_unverified_count: number;
    by_kind: {
      kind: string;
      approved_count: number;
      approved_cents: number;
      paid_count: number;
      paid_cents: number;
    }[];
    batches_total?: number;
    batches_truncated?: boolean;
    batches: {
      id: number;
      month: string;
      title: string;
      status: string;
      claim_count: number;
      total_cents: number;
      created_by_name: string;
      paid_by_name: string | null;
      paid_at: string | null;
      paid_ref: string;
      maker_checker_ok: boolean;
    }[];
    monthly_trend: {
      month: string;
      approved_count?: number;
      approved_cents: number;
      paid_count?: number;
      paid_cents: number;
      claim_count: number;
    }[];
  } | null;
  bills: {
    totals: {
      total_settlements: number;
      draft_count: number;
      published_count: number;
      closed_count: number;
      overdue_settlements_count?: number;
      settlements_total?: number;
      settlements_truncated?: boolean;
      gross_cents: number;
      subsidy_cents: number;
      net_billed_cents: number;
      collected_cents: number;
      pending_verification_cents: number;
      needs_info_cents: number;
      unpaid_cents: number;
      collection_rate_pct: number;
    };
    settlements: {
      id: number;
      title: string;
      event_id: number;
      event_title: string;
      event_starts_at: string;
      organiser_name: string;
      status: string;
      due_date?: string | null;
      gross_cents: number;
      subsidy_cents: number;
      net_billed_cents: number;
      collected_cents: number;
      outstanding_cents: number;
      participant_count: number;
      confirmed_count: number;
    }[];
  } | null;
}

export interface Reimb {
  id: number;
  user: { id: number; name: string };
  ride_id: number | null;
  ride_at: string | null;
  amount_cents: number;
  description: string;
  receipt_file_id: number;
  payee_name: string | null;
  payee_phone: string | null;
  status: string;
  review_note: string;
  reviewer: string | null;
  payee_verified: boolean;
  paid_ref: string | null;
  paid_at: string | null;
  created_at: string;
}

export interface PaginatedReimbursements {
  items: Reimb[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}
