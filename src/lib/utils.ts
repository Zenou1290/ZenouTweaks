/** Class name joiner (tiny clsx stand-in, no dependency). */
export function cn(
  ...parts: Array<string | number | false | null | undefined>
): string {
  return parts.filter((p) => typeof p === "string" || typeof p === "number").join(" ");
}
