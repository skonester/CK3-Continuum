import type { SaveInspection } from '../native/index';
export type Slot = 'source' | 'reference';
export interface InspectedSave {
  name: string;
  inspectedAt: string;
  inspection: SaveInspection;
}
export type Reply<T> = { ok: true; value: T } | { ok: false; error: string; cancelled?: boolean };
export interface ConversionSummary {
  outputName: string; reportName: string; profile: string; counts: Record<string, number>;
  outputBytes: number; outputSha256: string; warnings: string[]; engineTested: boolean;
}
export type ConversionMode = 'roundtrip' | 'experimental-1.16.1-to-1.19.0.6' | 'experimental-random-regions-1.16.1-to-1.19.0.6';
export interface DesktopBridge {
  convertSave(mode: ConversionMode): Promise<Reply<ConversionSummary | null>>;

  openSave(slot: Slot): Promise<Reply<InspectedSave | null>>;
  cancelInspection(): Promise<Reply<boolean>>;
  exportReport(): Promise<Reply<string | null>>;
}
declare global { interface Window { continuum?: DesktopBridge } }
