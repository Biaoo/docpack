import { createClient } from "../api/client";

export async function syncWorkspace(baseUrl: string): Promise<string> {
  const client = createClient({ baseUrl });
  const status = await client.getStatus();
  return status.ok ? "synced" : "failed";
}
