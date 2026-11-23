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

export interface MonthReport {
  user: { id: number; name: string };
  month: string;
  range: string;
  type: string;
  period_start: string;
  period_end: string;
  rides_driven: number;
  rides_received: number;
  distance_km: number;
  reimbursement_cents: number;
  events_attended: number;
  by_kind: { kind: string; count: number }[];
  events: { id: number; title: string; kind: string; starts_at: string }[];
  costs_paid_cents: number;
  badges: Badge[];
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
