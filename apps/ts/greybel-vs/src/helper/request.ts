export class JsonRequest {
  /** Posts `json` to `url` and returns the server's JSON response. */
  async post<T>(url: string, json: unknown): Promise<T> {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(json)
    });

    if (!response.ok) {
      const body = (await response.json().catch(() => undefined)) as { message?: unknown } | undefined;
      const reason = typeof body?.message === 'string' ? body.message : `${response.status} ${response.statusText}`;

      throw new Error(`${url} responded with ${reason}`);
    }

    return (await response.json()) as T;
  }
}
