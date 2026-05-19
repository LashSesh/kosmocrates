/* tslint:disable */
/* eslint-disable */

/**
 * The WASM-exposed PSE engine.
 * Wraps the real engine with a JSON-in/JSON-out interface.
 */
export class PseWasm {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get accumulation curve data.
     * Returns JSON array of { "tick": N, "total_crystals": N, "memory_hits": N }
     */
    accumulation_curve(): string;
    /**
     * Get all crystals as JSON array.
     */
    crystals(): string;
    /**
     * Ingest CSV data as a string.
     * Returns JSON: { "rows_ingested": N, "columns": N, "entities": N, "column_stats": [...] }
     */
    ingest_csv(csv_data: string): string;
    /**
     * Create a new PSE engine instance.
     */
    constructor();
    /**
     * Get quality report for ingested CSV data.
     * Returns JSON with anomalies, drift events, column stats.
     */
    quality_report(csv_data: string): string;
    /**
     * Full reset (clear everything including pattern memory).
     */
    reset_all(): void;
    /**
     * Reset observations (clear graph but keep crystals in memory).
     */
    reset_observations(): void;
    /**
     * Run N ticks of the engine with synthetic data.
     * Returns JSON: { "ticks_run": N, "new_crystals": N, "memory_hits": N, "time_ms": F }
     */
    run(ticks: number): string;
    /**
     * Get the embedded sample CSV data for "Try with sample data".
     */
    static sample_csv(): string;
    /**
     * Get engine status.
     * Returns JSON: { "observations": N, "crystals": N, "memory_size": N, "hit_rate": F }
     */
    status(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_psewasm_free: (a: number, b: number) => void;
    readonly psewasm_accumulation_curve: (a: number, b: number) => void;
    readonly psewasm_crystals: (a: number, b: number) => void;
    readonly psewasm_ingest_csv: (a: number, b: number, c: number, d: number) => void;
    readonly psewasm_new: () => number;
    readonly psewasm_quality_report: (a: number, b: number, c: number, d: number) => void;
    readonly psewasm_reset_all: (a: number) => void;
    readonly psewasm_reset_observations: (a: number) => void;
    readonly psewasm_run: (a: number, b: number, c: number) => void;
    readonly psewasm_sample_csv: (a: number) => void;
    readonly psewasm_status: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
