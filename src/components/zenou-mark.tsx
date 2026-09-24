import { type SVGProps } from "react";

/** Zenou "Z" logo mark — teal on near-black, matches icons-src/icon.svg. */
export function ZenouMark(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 64 64" fill="none" {...props}>
      <rect width="64" height="64" rx="14" fill="#0B0F10" />
      <path
        d="M18 20h28v6L28 42h18v6H18v-6l18-16H18v-6z"
        fill="#2DD4BF"
      />
    </svg>
  );
}
