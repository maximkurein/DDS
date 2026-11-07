// Mirrors of the Rust types serialized by dds-core (api.rs / regs.rs / model.rs).
// Keep in sync when adding RPC commands or snapshot fields.

export type Access = "rw" | "ro" | "sticky" | "fixed";

export type RegGroup = "common" | "ch1" | "lfm1" | "prof1" | "ch2" | "lfm2" | "prof2";

export interface RegDef {
  addr: number;
  name: string;
  title: string;
  reset: number;
  group: RegGroup;
}

export interface FieldDef {
  name: string;
  reg: number;
  msb: number;
  lsb: number;
  access: Access;
  default: number;
  title: string;
  desc: string;
  options: [number, string][];
  block: string;
  unit: string;
  signed: boolean;
}

export interface PinDef {
  name: string;
  dir: "in" | "out";
  idle: boolean;
  title: string;
}

export interface ChipInfo {
  name: string;
  vendor: string;
  board: string;
  bus: string;
  data_bits: number;
  can_read: boolean;
  commit_reg: number | null;
  /** Profiles shown for channel 1 and channel 2. */
  n_prof: [number, number];
}

export interface Meta {
  regs: RegDef[];
  fields: FieldDef[];
  defaults: number[];
  pins: PinDef[];
  version: string;
  chip: ChipInfo;
  /** Output chain elements, in order. */
  chain: string[];
}

export interface Metric {
  key: string;
  label: string;
  value: number;
  unit: string;
  block: string;
  top: boolean;
}

export interface Issue {
  level: "error" | "warn" | "info";
  block: string;
  text: string;
}

export interface ChanCalc {
  profile: number;
  ftw: number;
  fout: number;
  phase_deg: number;
  amp: number;
  offset: number;
  on_dac1: boolean;
  lfm: boolean;
}

export interface Calc {
  fclk: number;
  resolution: number;
  ch1: ChanCalc;
  ch2: ChanCalc;
  dac1_on: boolean;
  ifs: number;
  sinc_db: number;
  chain_db: number;
  p_out_dbm: number;
  p_fullscale_dbm: number;
  mul_for_target: number | null;
  image_hz: number;
  image_dbm: number;
  hd2_hz: number;
  hd2_dbm: number;
  dip_n: number | null;
  v_outm_max: number;
  i_1v8: number;
  p_d7: number;
}

export interface Derived {
  metrics: Metric[];
  issues: Issue[];
  calc: Calc;
}

/** Board-level settings (model.rs `Settings`). */
export interface Settings {
  fclk_hz: number;
  /** S1: 1 — такт с XW1, 0 — генератор G1. */
  csel: number;
  clk_dbm: number;
  rref_ohm: number;
  inductor_q: number;
  band_lo_hz: number;
  band_hi_hz: number;
  step_hz: number;
  target_dbm: number;
  sim_clock_ok: boolean;
  sim_pgood: boolean;
}

/** Board state from the firmware `ST` command (or the simulator). */
export interface BoardState {
  fw: string;
  remote: boolean;
  pgood: boolean;
  apply_pressed: boolean;
  dip: number | null;
  err: number;
  hop_n: number;
  hop_ms: number;
}

export interface LinkInfo {
  kind: "sim" | "serial";
  name: string;
  detail: string;
  readback: boolean;
}

export interface SimStatus {
  state: "ok" | "busy" | "warn" | "fault" | "off";
  text: string;
  signals: { name: string; value: number; unit: string }[];
  pins: [string, boolean][];
}

export interface Snapshot {
  regs: number[];
  desired: number[];
  dev_regs: number[] | null;
  dirty: boolean[];
  fields: Record<string, number>;
  derived: Derived;
  settings: Settings;
  link: LinkInfo;
  link_error: string | null;
  auto_apply: boolean;
  pins: Record<string, boolean>;
  inputs: Record<string, boolean | null>;
  log_head: number;
  readback: { words: (number | null)[]; matches: (boolean | null)[] };
  sim: SimStatus | null;
  board: BoardState | null;
  last_port: string | null;
}

export interface Status {
  sim: SimStatus | null;
  inputs: Record<string, boolean | null>;
  log_head: number;
  link_error: string | null;
  board: BoardState | null;
  /** Profile the simulator currently plays (firmware hopping). */
  live_profile: number | null;
}

export interface Txn {
  id: number;
  t_ms: number;
  op: "write" | "read" | "raw" | "pin" | "note" | "cmd";
  reg: number | null;
  addr: number | null;
  tx: string | null;
  rx: string | null;
  value: number | null;
  changes: [string, number, number][];
  note: string;
  ok: boolean;
}

export interface PortInfo {
  path: string;
  kind: string;
  vid: number | null;
  pid: number | null;
  manufacturer: string | null;
  product: string | null;
  serial: string | null;
}

export interface UsbDev {
  vid: number;
  pid: number;
  manufacturer: string | null;
  product: string | null;
  serial: string | null;
  bus: number;
  address: number;
  hint: string | null;
  likely_bridge: boolean;
}

export interface FreqPlan {
  target: number;
  ftw: number;
  ftw_hex: string;
  words: [number, number, number];
  actual: number;
  error_hz: number;
  resolution: number;
  dip_n: number | null;
  dip_on: boolean[];
  in_band: boolean;
  level_dbm: number;
}

export interface Profile {
  y: number;
  ftw: number;
  f: number;
  phase_deg: number;
  mul: number;
  offset: number;
}

export interface ResponseData {
  f: number[];
  chain_db: number[];
  sinc_db: number[];
  out_dbm: number[];
  fc_3db: number;
  band_ripple_db: number;
  elements: [string, string][];
}

export interface Tone {
  name: string;
  f: number;
  dbm: number;
  dbc: number;
  kind: "carrier" | "image" | "harmonic" | "spur";
}

export interface SpectrumData {
  f: number[];
  dbm: number[];
  dig_f: number[];
  dig_dbfs: number[];
  tones: Tone[];
  fs: number;
  n: number;
  rbw_hz: number;
  carrier_dbm: number;
  sfdr_db: number;
  sfdr_digital_db: number;
  samples: number[];
}

export interface PhaseNoiseData {
  offsets: number[];
  dbc_hz: number[];
  fout: number;
  jitter_s: number;
  ref_curves: [number, [number, number][]][];
}

export interface LfmReq {
  f_start: number;
  f_stop: number;
  t_ramp: number;
  t_hold: number;
  shape: "saw" | "tri" | "single";
  pause_off: boolean;
}

export interface LfmPlan {
  f_start: number;
  f_end: number;
  t_ramp: number;
  slope_hz_s: number;
  slope_step: number;
  df1: number;
  df2: number;
  tph: [number, number, number, number];
  fields: [string, number][];
  curve_t: number[];
  curve_f: number[];
  curve_on: boolean[];
  period: number;
  notes: string[];
}

export type ModKind = "fsk2" | "fsk4" | "bpsk" | "qpsk" | "psk8" | "ask2" | "ask4" | "qam16";

export interface ModReq {
  kind: ModKind;
  f_center: number;
  deviation: number;
  mul: number;
}

export interface ModSymbol {
  profile: number;
  bits: string;
  f: number;
  phase_deg: number;
  mul: number;
  i: number;
  q: number;
}

export interface ModPlan {
  kind: ModKind;
  title: string;
  bits_per_symbol: number;
  symbols: ModSymbol[];
  fields: [string, number][];
  sequence: number[];
  notes: string[];
}

export type ViewId =
  | "overview"
  | "board"
  | "registers"
  | "freq"
  | "spectrum"
  | "chain"
  | "sweep"
  | "hop"
  | "mod"
  | "hardware"
  | "learn";
