export interface ClientOptions {
  baseUrl: string;
  apiKey?: string;
}

export interface SampleClient {
  getStatus(): Promise<{ ok: true; baseUrl: string }>;
}

export function createClient(options: ClientOptions): SampleClient {
  return {
    async getStatus() {
      return {
        ok: true,
        baseUrl: options.baseUrl,
      };
    },
  };
}
