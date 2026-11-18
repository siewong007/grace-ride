export interface GuideStep {
  id?: string;
  title_en: string;
  title_zh: string;
  body_en: string;
  body_zh: string;
  link: string;
  image_file_id?: number | null;
  note_kind?: 'tip' | 'warning' | 'important' | 'note';
  note_en?: string;
  note_zh?: string;
}

export interface GuideSummary {
  id: number;
  slug: string;
  title_en: string;
  title_zh: string;
  summary_en: string;
  summary_zh: string;
  published: boolean;
  audience: 'member' | 'staff';
  step_count: number;
}

export interface GuideDetailData extends Omit<GuideSummary, 'step_count'> {
  steps: GuideStep[];
}

export interface TicketSummary {
  id: number;
  subject: string;
  user_id: number;
  user_name: string;
  status: string;
  ride_id: number | null;
  created_at: string;
  updated_at: string;
  last_body?: string | null;
  last_staff?: boolean | null;
  last_at?: string | null;
  assignee_id?: number | null;
  assignee_name?: string | null;
}

