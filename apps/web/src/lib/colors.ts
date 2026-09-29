// Stable colour slots for projects, shared with the desktop and Android
// clients: a small multiplicative hash over the project id's bytes.
export const PROJECT_COLORS = 8;
export function projectColorIndex(id: string): number {
  let hash = 0;
  for (let i = 0; i < id.length; i++)
    hash = (Math.imul(hash, 31) + id.charCodeAt(i)) >>> 0;
  return hash % PROJECT_COLORS;
}
