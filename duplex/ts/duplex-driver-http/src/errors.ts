export class ServerError extends Error {
  constructor(readonly status: number) {
    super(`the server responded ${status}`);
  }

  static ensureOk(response: Response): void {
    if (!response.ok) throw new ServerError(response.status);
  }

  static transportIdOf(response: Response, header: string): string {
    const transportId = response.headers.get(header);

    if (transportId === null) throw new ServerError(response.status);

    return transportId;
  }

  static getBody(response: Response): ReadableStream<Uint8Array> {
    ServerError.ensureOk(response);
    if (response.body === null) throw new ServerError(response.status);

    return response.body;
  }
}
