/* tslint:disable */
/* eslint-disable */

/**
 * Return a human-readable version banner identifying the loaded Alestia WASM build.
 */
export function alestia_version(): string;

/**
 * Expand Celestia 3-letter Greek Bayer abbreviations (e.g., `"ALF Cen"` -> `"α Cen"`), one designation per line.
 */
export function expand_greek_bayer(input: string): string;

/**
 * Sort newline-delimited astronomical catalog names using Celestia's natural case-insensitive sort.
 */
export function natural_sort_lines(input: string): string;

/**
 * Parse a Celestia catalog block (`{ Key Value ... }`) into a formatted property summary.
 *
 * # Errors
 * Returns a formatted [`alestia_util::ParseError`] message if `source` is not a valid catalog hash.
 */
export function parse_catalog_object(source: string): string;

/**
 * Parse a `#rrggbb` or `#rrggbbaa` hexadecimal color string into normalized sRGB and RGBA summary text.
 *
 * # Errors
 * Returns an error if `spec` is not a valid 6- or 8-digit hex color prefixed with `#`.
 */
export function parse_color_hex(spec: string): string;

/**
 * Decode an [`R128`] Base64 coordinate string back into `f64`.
 *
 * # Errors
 * Returns a descriptive error message when `encoded` is not a valid `R128` Base64 string.
 */
export function r128_decode_base64(encoded: string): number;

/**
 * Encode a floating-point coordinate using Alestia's 64.64 fixed-point [`R128`] Base64 codec.
 */
export function r128_encode_base64(value: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly alestia_version: () => [number, number];
    readonly expand_greek_bayer: (a: number, b: number) => [number, number];
    readonly natural_sort_lines: (a: number, b: number) => [number, number];
    readonly parse_catalog_object: (a: number, b: number) => [number, number, number, number];
    readonly parse_color_hex: (a: number, b: number) => [number, number, number, number];
    readonly r128_decode_base64: (a: number, b: number) => [number, number, number];
    readonly r128_encode_base64: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
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
