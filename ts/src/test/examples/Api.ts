const postHeaders: HeadersInit = {
  Accept: "application/json, text/plain, */*",
  "Content-Type": "application/octet-stream",
};

const getHeaders: HeadersInit = {
  Accept: "application/json, text/plain, */*",
};

export type RequestType = "GET" | "PUT" | "POST" | "PATCH";

function buildOptions(method: RequestType, headers: HeadersInit, entityBytes: Buffer | null) {
  const result: RequestInit = { method, headers, body: entityBytes };
  return result;
}

/**
 * Make a http get-request.
 *
 * @param url the url to request.
 * @param headers additional headers
 * @return a promise containing the result of the get request.
 */
export function getRequest(url: string, headers?: HeadersInit): Promise<Buffer | undefined> {
  const options = buildOptions("GET", { ...getHeaders, ...headers }, null);
  return handleFetch(fetch(url, options));
}

/**
 * Make a http put-request.
 *
 * @param url the url to request.
 * @param data the data to put.
 * @param headers additional headers
 * @return a promise containing whether the put succeeded or not.
 */
export function putRequest(url: string, data: Buffer, headers?: HeadersInit): Promise<boolean> {
  const options = buildOptions("PUT", { ...postHeaders, ...headers }, data);
  return fetch(url, options)
    .then((response) => response.ok)
    .catch(() => false);
}

function handleFetch(promise: Promise<Response>): Promise<Buffer | undefined> {
  return promise
    .then(async (response) => {
      if (response.status === 200) {
        return Buffer.from(await response.arrayBuffer());
      }
    })
    .catch(() => undefined);
}
