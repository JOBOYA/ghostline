export interface RunMeta {
  name: string;
  size: number;
}

export async function fetchRuns(): Promise<RunMeta[]> {
  try {
    const res = await fetch('/api/runs');
    if (!res.ok) return [];
    return res.json();
  } catch {
    return [];
  }
}

export async function fetchRunData(name: string): Promise<ArrayBuffer> {
  const res = await fetch(`/api/runs/${name}`);
  return res.arrayBuffer();
}
