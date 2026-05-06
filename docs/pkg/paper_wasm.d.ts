/* tslint:disable */
/* eslint-disable */

export class PaperTrader {
    free(): void;
    [Symbol.dispose](): void;
    buy(pair: string, price: number, amount: number): any;
    get_balances(): any;
    get_orders(): any;
    get_state(): any;
    get_status(): any;
    init(): any;
    load_state(json: string): any;
    constructor();
    reset(): any;
    save_state(): string;
    sell(pair: string, price: number, amount: number): any;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_papertrader_free: (a: number, b: number) => void;
    readonly papertrader_buy: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly papertrader_get_balances: (a: number) => any;
    readonly papertrader_get_orders: (a: number) => any;
    readonly papertrader_get_state: (a: number) => any;
    readonly papertrader_get_status: (a: number) => any;
    readonly papertrader_init: (a: number) => any;
    readonly papertrader_load_state: (a: number, b: number, c: number) => [number, number, number];
    readonly papertrader_new: () => number;
    readonly papertrader_save_state: (a: number) => [number, number];
    readonly papertrader_sell: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly papertrader_reset: (a: number) => any;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
