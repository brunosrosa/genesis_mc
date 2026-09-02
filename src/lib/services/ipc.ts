// SOULS MC — Unified High-Performance IPC Bridge (Zero-VDOM / Wry + Svelte 5)
//
// Conformidade: ADR-001, ADR-005, ADR-014, ADR-041.
// Roteia mensagens via `window.ipc.postMessage` (Wry) e despacha eventos tipados.

export interface TelemetrySnapshot {
  cpu_usage_percent: number;
  ram_used_mb: number;
  ram_total_mb: number;
  vram_used_mb: number;
  vram_total_mb: number;
  gpu_temperature_c: number;
  active_model: string;
  active_backend: string;
  tokens_per_sec: number;
  is_kill_switch_active: boolean;
  timestamp_epoch_ms: number;
}

export interface SocraticThoughtEvent {
  session_id: string;
  thought_id: string;
  iteration: number;
  max_iterations: number;
  branch_type: string;
  hypothesis: string;
  score: number;
  is_final: boolean;
  latency_ms: number;
}

export interface TerminalStreamEvent {
  id: string;
  stream_type: string;
  line: string;
  source_tag: string;
  timestamp_epoch_ms: number;
}

export interface BlastRadiusEvent {
  incident_id: string;
  blast_level: string;
  affected_subsystems: string[];
  human_in_the_loop_required: boolean;
  is_kill_switch_active: boolean;
  reason: string;
}

type EventCallback = (payload: any) => void;

interface PendingPromise {
  resolve: (value: any) => void;
  reject: (reason: any) => void;
  timeoutId: ReturnType<typeof setTimeout>;
}

class SoulsIpcClient {
  private pendingRequests = new Map<string, PendingPromise>();
  private eventListeners = new Map<string, Set<EventCallback>>();
  private isWryAvailable = false;

  constructor() {
    if (typeof window !== "undefined") {
      this.isWryAvailable = !!(window as any).ipc?.postMessage;

      // Injeta os hooks globais de retorno de chamada
      (window as any).__SOULS_DISPATCH__ = (id: string, response: any) => {
        const pending = this.pendingRequests.get(id);
        if (pending) {
          clearTimeout(pending.timeoutId);
          this.pendingRequests.delete(id);
          if (response.status === "Ok") {
            pending.resolve(response.data);
          } else if (response.status === "Error") {
            pending.reject(new Error(`[${response.data?.code}] ${response.data?.message}`));
          } else {
            pending.resolve(response);
          }
        }
      };

      (window as any).__SOULS_EVENT__ = (channel: string, payload: any) => {
        const listeners = this.eventListeners.get(channel);
        if (listeners) {
          for (const listener of listeners) {
            try {
              listener(payload);
            } catch (err) {
              console.error(`[SoulsIPC] Erro no listener do canal ${channel}:`, err);
            }
          }
        }
      };
    }
  }

  /**
   * Indica se o runtime nativo do Wry IPC está presente
   */
  public get isAvailable(): boolean {
    return this.isWryAvailable;
  }

  /**
   * Envia comando assíncrono para o backend Rust via Wry IPC
   */
  public async invoke<T = any>(action: string, data: any = {}): Promise<T> {
    const id = `req_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
    const envelope = {
      id,
      channel: "command",
      payload: { action, data },
    };

    if (this.isWryAvailable && typeof window !== "undefined") {
      return new Promise<T>((resolve, reject) => {
        const timeoutId = setTimeout(() => {
          this.pendingRequests.delete(id);
          reject(new Error(`[SoulsIPC] Timeout ao aguardar resposta para ação '${action}' (ID: ${id})`));
        }, 8000);

        this.pendingRequests.set(id, { resolve, reject, timeoutId });
        (window as any).ipc.postMessage(JSON.stringify(envelope));
      });
    }

    // Fallback gracioso para ambiente de desenvolvimento local / mock
    console.debug(`[SoulsIPC:Mock] invoke '${action}':`, data);
    if (action === "Ping") {
      return { engine: "souls_core_mock", status: "online", version: "0.1.0" } as any;
    }
    if (action === "RequestTelemetrySnapshot") {
      return {
        cpu_usage_percent: 12.4,
        ram_used_mb: 4200,
        ram_total_mb: 16384,
        vram_used_mb: 1840,
        vram_total_mb: 6144,
        gpu_temperature_c: 46.5,
        active_model: "BitNet-b1.58-2B-Q4",
        active_backend: "Candle/AVX2",
        tokens_per_sec: 42.0,
        is_kill_switch_active: false,
        timestamp_epoch_ms: Date.now(),
      } as any;
    }
    return { acknowledged: true } as any;
  }

  /**
   * Registra um ouvinte para canais de broadcast de eventos
   */
  public onEvent<T = any>(channel: string, callback: (payload: T) => void): () => void {
    if (!this.eventListeners.has(channel)) {
      this.eventListeners.set(channel, new Set());
    }
    const listeners = this.eventListeners.get(channel)!;
    listeners.add(callback);

    return () => {
      listeners.delete(callback);
      if (listeners.size === 0) {
        this.eventListeners.delete(channel);
      }
    };
  }

  /**
   * Emite um evento mock para testes no frontend
   */
  public emitMockEvent(channel: string, payload: any): void {
    if (typeof window !== "undefined" && (window as any).__SOULS_EVENT__) {
      (window as any).__SOULS_EVENT__(channel, payload);
    }
  }
}

export const soulsIpc = new SoulsIpcClient();
