/* tslint:disable */
/* eslint-disable */

/**
 * Parse a spec once and render it many times against different
 * state values. Avoids re-parsing JSON + revalidating the catalog
 * on each frame. Call `.free()` to release the Rust-side AST.
 */
export class Template {
    free(): void;
    [Symbol.dispose](): void;
    constructor(spec_json: string);
    render(state_json: string): string;
}

/**
 * One-shot: `compile(spec, state) -> html` or throws. Use when the
 * spec changes every render anyway.
 */
export function compile(spec_json: string, state_json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_template_free: (a: number, b: number) => void;
    readonly compile: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly template_new: (a: number, b: number, c: number) => void;
    readonly template_render: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
