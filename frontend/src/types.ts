export type DiffKind = "none" | "light" | "fatal";

export type Phoneme = {
  value: string;
  diff: DiffKind;
};

export type Stats = {
  total: number;
  characters: number;
  matches: number;
  lightMismatches: number;
  fatalMismatches: number;
  jpErrors: number;
  ojtErrors: number;
  openjtalkExtractionDurationMs: number;
  openjtalkThroughputCharsPerSecond: number;
  jpreprocessExtractionDurationMs: number;
  jpreprocessThroughputCharsPerSecond: number;
};

export type MatchEntry = {
  kind: "match";
  index: number;
  original: string;
  openjtalk: Phoneme[];
  jpreprocess: Phoneme[];
};

export type MismatchEntry = {
  kind: "light" | "fatal";
  index: number;
  original: string;
  openjtalk: Phoneme[];
  jpreprocess: Phoneme[];
  lengthMismatch?: boolean;
};

export type ErrorEntry = {
  kind: "jp_error" | "ojt_error" | "both_error" | "jp_panic";
  index: number;
  original: string;
  openjtalkError?: string;
  jpreprocessError?: string;
};

export type Entry = MatchEntry | MismatchEntry | ErrorEntry;

export type FileResult = {
  file: string;
  stats: Stats;
  entries: Entry[];
};

export type Results = {
  generatedAt: string;
  commit: string;
  totals: Stats;
  files: FileResult[];
};
