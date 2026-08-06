// web-probe extension for rho
// Checks the status of external websites and APIs

export async function probeUrl(url: string, options?: { timeout?: number; followRedirects?: boolean }): Promise<ProbeResult> {
  const defaultOptions = {
    timeout: 5000,
    followRedirects: true
  };

  try {
    // Use fetch with AbortController for timeout control
    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), options?.timeout ?? defaultOptions.timeout);

    const response = await fetch(url, {
      method: 'HEAD',
      headers: {
        'User-Agent': 'rho-web-probe/1.0'
      },
      redirect: options?.followRedirects ? 'follow' : 'manual',
      signal: controller.signal
    });

    clearTimeout(id);

    const startTime = Date.now();
    const endTime = Date.now();

    // For HEAD requests, we get status and headers but no body by default
    // If you want to fetch the body as well:
    // let body; try { body = await response.text(); } catch {} (handled internally)

    return {
      url,
      status: response.status,
      ok: response.ok,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      contentType: response.headers.get('content-type'),
      size: parseInt(response.headers.get('content-length') || '0'),
      latencyMs: endTime - startTime
    };
  } catch (error) {
    return {
      url,
      status: null,
      ok: false,
      error: error instanceof Error ? error.message : 'Unknown error',
      latencyMs: Date.now()
    };
  }
}

export async function batchProbe(urls: string[], options?: { timeout?: number }): Promise<ProbeResult[]> {
  const results = await Promise.allSettled(
    urls.map(url => probeUrl(url, options))
  );

  return results.map(result => {
    if (result.status === 'fulfilled') {
      return result.value;
    }
    // If we want to handle fetch failures differently:
    return {
      url,
      status: null,
      ok: false,
      error: 'Network failure',
      latencyMs: Date.now()
    };
  });
}

export async function isAlive(url: string, options?: { timeout?: number }): Promise<boolean> {
  const result = await probeUrl(url, options);
  return !!result.status && result.ok;
}

// Type definitions
interface ProbeResult {
  url: string;
  status: number | null;
  ok: boolean;
  statusText?: string;
  headers?: Record<string, string>;
  contentType?: string;
  size?: number;
  latencyMs: number;
  error?: string;
}

// Simple example usage (this would be used by rho via some plugin system):
/*
const results = await probeUrl('https://api.example.com/health', { timeout: 2000 });
console.log(`Status: ${results.status}, Latency: ${results.latencyMs}ms`);

if (results.error) {
  console.warn(`${results.url}: ${results.error}`);
}
*/

export default probeUrl;
