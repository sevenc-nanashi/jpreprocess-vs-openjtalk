export type DiffKind = "none" | "light" | "fatal";

export type Phoneme = {
  value: string;
  diff: DiffKind;
};

export type Phrase = {
  text: string;
  reading: string;
  diff?: DiffKind;
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
  openjtalkPhrases?: Phrase[];
  jpreprocessPhrases?: Phrase[];
};

export type MismatchEntry = {
  kind: "light" | "fatal";
  index: number;
  original: string;
  openjtalk: Phoneme[];
  jpreprocess: Phoneme[];
  openjtalkPhrases?: Phrase[];
  jpreprocessPhrases?: Phrase[];
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
  chunks?: EntryChunk[];
};

export type EntryChunk = {
  path: string;
  counts: { matches: number; light: number; fatal: number; errors: number };
};

export type Results = {
  generatedAt: string;
  commit: string;
  totals: Stats;
  files: FileResult[];
};
