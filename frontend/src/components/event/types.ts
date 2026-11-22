import type { EventItem } from '../../pages/Events';

export interface EventFull extends EventItem {
  group_ids: number[];
  /** Groups this member may join as (best match first). */
  join_as?: { id: number; name: string }[];
  my_ride: { id: number; status: string } | null;
  my_return_ride?: { id: number; status: string } | null;
  settlement: { id: number; status: string } | null;
}

export interface Signup {
  id: number;
  user_id: number;
  name: string;
  status: string;
  need_ride: boolean;
  offer_drive: boolean;
  offer_seats: number;
  attended: number | null;
  ride_status: string | null;
  as_group?: string | null;
}
