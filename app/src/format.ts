const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 1536 -> "1.5 KB" */
export function bytes(n: number): string {
  let i = 0;
  let v = n;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? v.toFixed(0) : v.toFixed(1)} ${UNITS[i]}`;
}

/** Bytes per second. */
export const bps = (n: number) => `${bytes(n)}/s`;

export const clock = (mhz: number) => (mhz >= 1000 ? `${(mhz / 1000).toFixed(2)} GHz` : `${mhz} MHz`);

/** 93784 -> "1d 2h" ; 3700 -> "1h 1m" */
export function duration(s: number): string {
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d) return `${d}d ${h}h`;
  if (h) return `${h}h ${m}m`;
  return `${m}m`;
}
