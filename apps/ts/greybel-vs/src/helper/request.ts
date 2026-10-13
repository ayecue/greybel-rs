/** Posts `json` to `url` and returns the server's JSON response. */
export async function postJson<T>(url: string, json: unknown): Promise<T> {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(json)
  });

  if (!response.ok) throw new Error(`${url} responded with ${response.status} ${response.statusText}`);

  return (await response.json()) as T;
}
