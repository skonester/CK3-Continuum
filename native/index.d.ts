ï»¿export interface MetadataField { key: string; value: string }
export interface SectionSummary {
  key: string;
  occurrences: number;
  /** Sum of immediate entries over every block occurrence; includes tombstones and list atoms. */
  childEntries: number;
}
export interface SaveInspection {
  format: string;
  fileBytes: number;
  gamestateBytes: number;
  metadataBytes: number;
  gamestateSha256: string;
  /** ZIP gamestate CRC and size checked; false for plain text, which has no ZIP CRC. */
  crcVerified: boolean;
  /** Scalar fields in inner meta_data, retained as ordered entries (duplicates not collapsed). */
  metadata: MetadataField[];
  sections: SectionSummary[];
  tokenCount: number;
  maxDepth: number;
  elapsedMs: number;
}
/** Read-only, asynchronous Rust streaming inspection. Not a conversion or an engine compatibility test. */
export function inspectSave(filePath: string, maxBytes?: number): Promise<SaveInspection>;

export interface ConversionChange { rule: string; path: string; start: number; end: number; replacementBytes: number; beforeSha256: string; afterSha256: string }
export interface ConversionResult {
  profile: string; sourceSha256: string; outputSha256: string;
  sourceGamestateSha256: string; outputGamestateSha256: string;
  outputBytes: number; unchangedSpansVerified: boolean; outputVerified: boolean;
  elapsedMs: number; changes: ConversionChange[]; referenceSha256?: string;
}
/** Creates a new staged file, never replaces an existing path. Experimental; not engine-validated. */
export function writeCandidate(input: string, output: string, expected: string, mode: 'roundtrip' | 'experimental-1.16.1-to-1.19.0.6' | 'experimental-random-regions-1.16.1-to-1.19.0.6', reference?: string): Promise<ConversionResult>;
