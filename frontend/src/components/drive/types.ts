export interface Area {
  id: number;
  name: string;
  active: boolean;
}

export interface Vehicle {
  id?: number;
  model: string;
  color: string;
  plate: string;
  seats: number;
  vehicle_type: string;
  type?: string;
  wheelchair_ok?: boolean;
  low_step?: boolean;
  child_seat?: boolean;
  is_active?: boolean;
}

export interface Profile {
  carpool_enabled: boolean;
  spare_seats: number;
  gate_wait_mins?: number;
  area_ids: number[];
  vehicle: Vehicle | null;
  vehicles?: Vehicle[];
  active_vehicle_id?: number | null;
}

export interface DriverStanding {
  id: number;
  kind: 'passenger' | 'driver';
  label: string;
  weekday: number;
  pickup_time: string;
  area_id: number;
  area_name: string;
  destination: string;
  seats: number;
}

export const WEEKDAYS_DRIVE = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'] as const;
